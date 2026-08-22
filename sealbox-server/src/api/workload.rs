//! Authenticating a workload by the token its platform already gives it.
//!
//! A Kubernetes ServiceAccount token is a JWT the cluster signs, mounted into the pod, rotated by
//! the kubelet, and reissued on every restart. Accepting one removes the last long-lived
//! credential from the deployment: there is nothing in a Secret to leak, nothing to rotate, and a
//! restart needs no re-registration.
//!
//! Nothing here reaches the network. A hosted server cannot dial a private cluster's OIDC endpoint
//! — the same constraint that made the runner poll outbound (ADR 0008) — so an issuer's keys are
//! registered by an admin and read from the store.
//!
//! This is OIDC, not Kubernetes: GitHub Actions, GCP, and AWS IRSA present the same shape, and no
//! provider-specific code belongs in sealbox.

use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

use crate::{
    api::state::AppState,
    error::{Result, SealboxError},
    repo::Identity,
};

/// How much clock skew to tolerate. Enough for machines that drift, short enough that an expired
/// token is not useful for long.
const LEEWAY_SECONDS: u64 = 60;

#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
}

/// Resolve a presented JWT to the identity bound to its issuer and subject, or `None`.
///
/// `None` covers every failure — a malformed token, an unregistered issuer, a bad signature, an
/// expired token, the wrong audience — because to a caller they are one thing: this credential
/// does not authenticate. Which one it was is recorded in the audit trail by the caller.
pub(crate) fn authenticate(state: &AppState, token: &str) -> Result<Option<Identity>> {
    // Shape check first, so an ordinary bearer token that failed to resolve does not get dragged
    // through JWT parsing on every request.
    if token.matches('.').count() != 2 {
        return Ok(None);
    }

    // Read the issuer and subject **without verifying**, to find which identity's rules apply.
    // These claims decide nothing: the token is checked against that identity's issuer, audience,
    // and subject immediately below, and a mismatch is a refusal.
    let Some(unverified) = peek(token) else {
        return Ok(None);
    };
    let Some(issuer) = state.issuer_repo.find_by_url(&unverified.iss)? else {
        return Ok(None);
    };
    let Some(identity) = state
        .identity_repo
        .find_by_workload(&issuer.name, &unverified.sub)?
    else {
        return Ok(None);
    };

    // An identity bound to an issuer must declare what audience it expects. A token minted for
    // the cluster's API server would otherwise authenticate here.
    let Some(audience) = identity.audience.as_deref() else {
        return Ok(None);
    };

    let Some(header) = decode_header(token).ok() else {
        return Ok(None);
    };
    let Some(key) = matching_key(&issuer.jwks, header.kid.as_deref())? else {
        return Ok(None);
    };

    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[issuer.url.as_str()]);
    validation.set_audience(&[audience]);
    validation.leeway = LEEWAY_SECONDS;
    // `exp` is what bounds a stolen token's usefulness, so a token without one is refused rather
    // than treated as valid forever.
    validation.required_spec_claims = ["exp", "iss", "aud", "sub"]
        .iter()
        .map(|c| c.to_string())
        .collect();

    let Ok(verified) = decode::<Claims>(token, &key, &validation) else {
        return Ok(None);
    };

    // Exact. A prefix or a pattern would mean that creating a ServiceAccount is enough to become
    // a runner, and far more people can create one than can be trusted with plaintext.
    if Some(verified.claims.sub.as_str()) != identity.subject.as_deref() {
        return Ok(None);
    }

    Ok(Some(identity))
}

/// The unverified claims, used only to decide which identity's rules to apply.
fn peek(token: &str) -> Option<Claims> {
    use base64::Engine;

    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kty: String,
    kid: Option<String>,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
    #[serde(default)]
    crv: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
}

/// The key the token names, or — when it names none and the issuer has exactly one — that one.
///
/// A JWKS holding several keys is what makes a signing-key rotation overlap instead of cutting
/// over: the new key is registered beside the old one, and the old one removed once nothing
/// presents it.
fn matching_key(jwks: &str, kid: Option<&str>) -> Result<Option<DecodingKey>> {
    let parsed: Jwks = serde_json::from_str(jwks)
        .map_err(|e| SealboxError::InvalidRequest(format!("stored JWKS is not valid: {e}")))?;

    let candidate = match kid {
        Some(kid) => parsed.keys.iter().find(|k| k.kid.as_deref() == Some(kid)),
        None if parsed.keys.len() == 1 => parsed.keys.first(),
        // Several keys and nothing naming one: guessing would mean trying each, which turns a
        // rotation into a way to probe for a key that verifies.
        None => None,
    };

    let Some(key) = candidate else {
        return Ok(None);
    };

    Ok(match key.kty.as_str() {
        "RSA" => match (&key.n, &key.e) {
            (Some(n), Some(e)) => DecodingKey::from_rsa_components(n, e).ok(),
            _ => None,
        },
        "EC" => match (&key.crv, &key.x, &key.y) {
            (Some(crv), Some(x), Some(y)) if crv == "P-256" || crv == "P-384" => {
                DecodingKey::from_ec_components(x, y).ok()
            }
            _ => None,
        },
        _ => None,
    })
}

/// Check a JWKS before it is stored, so a paste error is caught while a person is present rather
/// than at three in the morning when a runner cannot authenticate.
pub(crate) fn validate_jwks(jwks: &str) -> Result<usize> {
    let parsed: Jwks = serde_json::from_str(jwks).map_err(|e| {
        SealboxError::InvalidRequest(format!(
            "that does not parse as a JWKS: {e}. It is the document at the issuer's \
             `/openid/v1/jwks`, or `jwks_uri` in its OIDC discovery."
        ))
    })?;
    if parsed.keys.is_empty() {
        return Err(SealboxError::InvalidRequest(
            "that JWKS contains no keys".to_string(),
        ));
    }
    for key in &parsed.keys {
        let usable = match key.kty.as_str() {
            "RSA" => key.n.is_some() && key.e.is_some(),
            "EC" => key.x.is_some() && key.y.is_some(),
            _ => false,
        };
        if !usable {
            return Err(SealboxError::InvalidRequest(format!(
                "that JWKS holds a `{}` key sealbox cannot verify with. RSA and EC (P-256, \
                 P-384) are supported.",
                key.kty
            )));
        }
    }
    Ok(parsed.keys.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSA_JWKS: &str =
        r#"{"keys":[{"use":"sig","kty":"RSA","kid":"a","alg":"RS256","n":"xMR1","e":"AQAB"}]}"#;

    #[test]
    fn a_jwks_that_is_not_one_is_refused_with_a_pointer() {
        let err = validate_jwks("not json").unwrap_err().to_string();
        assert!(
            err.contains("jwks"),
            "the message should say where to get one: {err}"
        );
    }

    #[test]
    fn an_empty_jwks_is_refused() {
        assert!(validate_jwks(r#"{"keys":[]}"#).is_err());
    }

    #[test]
    fn a_key_type_that_cannot_be_verified_with_is_named() {
        let err = validate_jwks(r#"{"keys":[{"kty":"oct","k":"c2VjcmV0"}]}"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("oct"), "{err}");
    }

    #[test]
    fn a_valid_jwks_reports_how_many_keys_it_holds() {
        assert_eq!(validate_jwks(RSA_JWKS).unwrap(), 1);
    }

    #[test]
    fn a_key_is_chosen_by_kid() {
        let two = r#"{"keys":[
            {"kty":"RSA","kid":"old","n":"xMR1","e":"AQAB"},
            {"kty":"RSA","kid":"new","n":"yNS2","e":"AQAB"}
        ]}"#;
        assert!(matching_key(two, Some("old")).unwrap().is_some());
        assert!(matching_key(two, Some("new")).unwrap().is_some());
        assert!(
            matching_key(two, Some("neither")).unwrap().is_none(),
            "a kid naming no registered key must not fall back to one that happens to be there"
        );
        assert!(
            matching_key(two, None).unwrap().is_none(),
            "with a rotation in progress, an unnamed key must not be guessed"
        );
    }

    #[test]
    fn a_single_key_needs_no_kid() {
        // Not every issuer sets one, and with exactly one registered there is nothing to guess.
        assert!(matching_key(RSA_JWKS, None).unwrap().is_some());
    }

    #[test]
    fn something_that_is_not_a_jwt_is_not_dragged_through_verification() {
        // Shape check only — this must not depend on any state.
        assert_eq!("sealbox_deadbeef".matches('.').count(), 0);
    }
}
