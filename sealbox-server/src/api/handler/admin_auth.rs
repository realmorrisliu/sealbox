//! Enrolment, authentication, and the approval ceremony.
//!
//! The pages here are not an interface (ADR 0004). They exist because a terminal cannot be a
//! trusted display: its output is written by whatever process is running, so an agent can print
//! one grant's declaration and submit another. A page the server renders cannot be influenced
//! that way, so what a human reads is what they sign.
//!
//! Nothing here may grow a way to *manage* anything. The moment it lists secrets, it has become
//! the web UI that ADR 0004 rejects.

use axum::{
    extract::{Json, State},
    response::Html,
};
use serde_json::json;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    api::{
        SealboxResponse,
        passkey::{AuthState, PendingApproval},
        path::Path,
        state::AppState,
    },
    error::{Result, SealboxError},
    repo::{AuditOutcome, NewAuditRecord, Role},
};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub(crate) struct IdParam {
    id: Uuid,
}

// ---------------------------------------------------------------- enrolment

/// GET /enrol/{id}
pub(crate) async fn enrol_page(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
) -> Result<Html<String>> {
    let identity = state
        .passkey
        .peek_enrolment(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("this enrolment link has expired".into()))?;

    Ok(Html(page(
        "Register a passkey",
        &format!(
            "<p>Registering an authenticator for <strong>{}</strong>.</p>\
             <p class=note>After this, nothing on your machine will authenticate as an admin — \
             approving a capability will need this authenticator, and you.</p>",
            escape(&identity)
        ),
        "Register",
        &format!("/enrol/{}", params.id),
        "register",
    )))
}

/// POST /enrol/{id}/start
pub(crate) async fn enrol_start(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
) -> Result<SealboxResponse> {
    let identity = state
        .passkey
        .peek_enrolment(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("this enrolment link has expired".into()))?;

    // A link that already has an authenticator behind it must not work: a leaked link should be
    // a way to become *an* admin for the first time, never a way to displace a working one.
    if state.authenticator_repo.count_for(&identity)? > 0 {
        return Err(SealboxError::InvalidRequest(
            "that identity already has an authenticator registered".into(),
        ));
    }

    let (challenge, registration) = state
        .passkey
        .webauthn
        .start_passkey_registration(Uuid::new_v4(), &identity, &identity, None)
        .map_err(|e| SealboxError::InvalidRequest(format!("could not start registration: {e}")))?;

    state.passkey.stash_registration(params.id, registration);
    Ok(SealboxResponse::Json(json!(challenge)))
}

/// POST /enrol/{id}/finish
pub(crate) async fn enrol_finish(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
    Json(credential): Json<RegisterPublicKeyCredential>,
) -> Result<SealboxResponse> {
    let registration = state
        .passkey
        .take_registration(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("that challenge has expired".into()))?;
    let identity = state
        .passkey
        .consume_enrolment(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("this enrolment link has expired".into()))?;

    let passkey = state
        .passkey
        .webauthn
        .finish_passkey_registration(&credential, &registration)
        .map_err(|e| SealboxError::InvalidRequest(format!("registration failed: {e}")))?;

    let credential_id = base64_id(passkey.cred_id().as_ref());
    let serialised =
        serde_json::to_string(&passkey).map_err(|e| SealboxError::DatabaseError(e.to_string()))?;
    state
        .authenticator_repo
        .register(&identity, &credential_id, &serialised)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.clone()),
        action: "admin.enrol".to_string(),
        resource: None,
        outcome: AuditOutcome::Allowed,
        detail: Some("authenticator registered".to_string()),
    })?;

    Ok(SealboxResponse::Json(
        json!({ "identity": identity, "registered": true }),
    ))
}

// ---------------------------------------------------------------- login

/// POST /v1/auth/login — a CLI asks for a login it can wait on.
///
/// Public, and it grants nothing: it returns an unguessable id and the URL to open. What comes
/// back to the CLI later is a session, and only a signature puts one there.
pub(crate) async fn login_open(State(state): State<AppState>) -> Result<SealboxResponse> {
    let id = state.passkey.open_login();
    Ok(SealboxResponse::Json(json!({
        "login": id,
        "url": format!("{}/login/{id}", state.config.public_url),
    })))
}

/// GET /login/{id} — the page that does the ceremony.
pub(crate) async fn login_page(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
) -> Result<Html<String>> {
    if !state.passkey.login_is_open(&params.id) {
        return Err(SealboxError::InvalidRequest(
            "that sign-in request has expired".into(),
        ));
    }
    Ok(Html(page(
        "Sign in",
        "<h2>Sign in</h2>\
         <p class=lead>This opens an admin session in the terminal that asked for it.</p>\
         <p class=note>The session lives in that process's memory, expires on its own, and is \
         never written down.</p>",
        "Sign in",
        &format!("/login/{}", params.id),
        "authenticate",
    )))
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityPayload {
    identity: String,
}

/// POST /login/{id}/start
pub(crate) async fn login_start(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
    Json(payload): Json<IdentityPayload>,
) -> Result<SealboxResponse> {
    if !state.passkey.login_is_open(&params.id) {
        return Err(SealboxError::InvalidRequest(
            "that sign-in request has expired".into(),
        ));
    }
    let (challenge, id) = begin_authentication(&state, &payload.identity, None)?;
    Ok(SealboxResponse::Json(
        json!({ "challenge_id": id, "options": challenge }),
    ))
}

/// POST /login/{id}/finish — verify, and hand the session to the waiting CLI.
pub(crate) async fn login_finish(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
    Json(payload): Json<AuthFinishPayload>,
) -> Result<SealboxResponse> {
    let auth = finish_authentication(&state, payload.challenge_id, &payload.credential)?;

    // A challenge issued to approve a grant is not a way to open a session: the human was shown
    // a declaration, not a sign-in.
    if auth.approval.is_some() {
        return Err(SealboxError::InvalidRequest(
            "that challenge was issued to approve something, not to open a session".into(),
        ));
    }

    let identity = state
        .identity_repo
        .find_by_name(&auth.identity)?
        .ok_or(SealboxError::Unauthorized)?;
    if identity.role != Role::Admin {
        return Err(SealboxError::Forbidden);
    }

    let token = state.passkey.issue_session(&auth.identity);
    if !state.passkey.complete_login(&params.id, token) {
        return Err(SealboxError::InvalidRequest(
            "that sign-in request has expired".into(),
        ));
    }

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(auth.identity.clone()),
        action: "admin.session".to_string(),
        resource: None,
        outcome: AuditOutcome::Allowed,
        detail: Some("session opened".to_string()),
    })?;

    Ok(SealboxResponse::Json(json!({ "identity": auth.identity })))
}

/// GET /v1/auth/login/{id} — the waiting CLI collects its session.
pub(crate) async fn login_collect(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
) -> Result<SealboxResponse> {
    match state.passkey.collect_login(&params.id) {
        Some(session) => Ok(SealboxResponse::Json(json!({ "session": session }))),
        None => Ok(SealboxResponse::Json(json!({ "session": null }))),
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthFinishPayload {
    challenge_id: Uuid,
    credential: PublicKeyCredential,
}

fn begin_authentication(
    state: &AppState,
    identity: &str,
    approval: Option<Uuid>,
) -> Result<(RequestChallengeResponse, Uuid)> {
    let registered = state.authenticator_repo.for_identity(identity)?;
    if registered.is_empty() {
        return Err(SealboxError::InvalidRequest(format!(
            "`{identity}` has no registered authenticator"
        )));
    }

    let passkeys: Vec<Passkey> = registered
        .iter()
        .filter_map(|a| serde_json::from_str(&a.passkey).ok())
        .collect();

    let (challenge, auth_state) = state
        .passkey
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| {
            SealboxError::InvalidRequest(format!("could not start authentication: {e}"))
        })?;

    let id = Uuid::new_v4();
    state.passkey.stash_authentication(
        id,
        AuthState {
            identity: identity.to_string(),
            state: auth_state,
            approval,
        },
    );
    Ok((challenge, id))
}

fn finish_authentication(
    state: &AppState,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
) -> Result<AuthState> {
    // Taking removes it, so a replayed signature finds nothing.
    let auth = state
        .passkey
        .take_authentication(&challenge_id)
        .ok_or_else(|| {
            SealboxError::InvalidRequest("that challenge has expired or was already used".into())
        })?;

    state
        .passkey
        .webauthn
        .finish_passkey_authentication(credential, &auth.state)
        .map_err(|e| SealboxError::InvalidRequest(format!("authentication failed: {e}")))?;

    Ok(auth)
}

// ---------------------------------------------------------------- approval

/// GET /approve/{id} — the trusted display.
pub(crate) async fn approve_page(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
) -> Result<Html<String>> {
    let pending = state
        .passkey
        .peek_approval(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("that approval has expired".into()))?;

    // Rendered from what the server stored, never from anything the caller sent — otherwise an
    // agent could show one thing and have another signed, which is the attack this page exists
    // to prevent.
    Ok(Html(page(
        "Approve a grant",
        &render_declaration(&pending),
        "Approve",
        &format!("/approve/{}", params.id),
        "authenticate",
    )))
}

/// The secrets line first: sealbox confines the implementation to exactly those, so that is what
/// the approval is actually about. A script's body is deliberately not shown — judging one is a
/// hard cognitive task, and that kind of review decays into a glance.
fn render_declaration(pending: &PendingApproval) -> String {
    let payload = &pending.payload;
    let name = payload["name"].as_str().unwrap_or("(unnamed)");
    let runner = payload["runner"].as_str().unwrap_or("(none)");

    let mut secrets = String::new();
    for source in ["secrets", "files"] {
        if let Some(map) = payload[source].as_object() {
            for (injected, secret) in map {
                secrets.push_str(&format!(
                    "<li><code>{}</code> <span class=note>as {}</span></li>",
                    escape(secret.as_str().unwrap_or_default()),
                    escape(injected)
                ));
            }
        }
    }
    if secrets.is_empty() {
        secrets = "<li class=note>none</li>".to_string();
    }

    let implementation = if let Some(adapter) = payload["adapter"].as_str() {
        format!(
            "<p>Implementation: <code>{}</code> — a built-in, which cannot do anything beyond \
             what it implements.</p>",
            escape(adapter)
        )
    } else {
        "<p>Implementation: <strong>a custom script</strong> — it can do anything the secrets \
         below permit.</p>"
            .to_string()
    };

    format!(
        "<h2>{}</h2>\
         <p class=lead>Approving this lets it use these secrets, and nothing else:</p>\
         <ul class=secrets>{}</ul>\
         {}\
         <p>Runs on: <code>{}</code></p>\
         <p class=note>Requested by {}.</p>",
        escape(name),
        secrets,
        implementation,
        escape(runner),
        escape(&pending.requested_by)
    )
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApproveStartPayload {
    identity: String,
}

/// POST /approve/{id}/start
pub(crate) async fn approve_start(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
    Json(payload): Json<ApproveStartPayload>,
) -> Result<SealboxResponse> {
    state
        .passkey
        .peek_approval(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("that approval has expired".into()))?;

    let (challenge, id) = begin_authentication(&state, &payload.identity, Some(params.id))?;
    Ok(SealboxResponse::Json(
        json!({ "challenge_id": id, "options": challenge }),
    ))
}

/// POST /approve/{id}/finish — verify, then create the grant.
pub(crate) async fn approve_finish(
    State(state): State<AppState>,
    Path(params): Path<IdParam>,
    Json(payload): Json<AuthFinishPayload>,
) -> Result<SealboxResponse> {
    let auth = finish_authentication(&state, payload.challenge_id, &payload.credential)?;

    // The signature is bound to the approval the challenge was issued for. A mismatch means
    // something is being substituted.
    if auth.approval != Some(params.id) {
        return Err(SealboxError::InvalidRequest(
            "that signature was not made for this approval".into(),
        ));
    }

    let identity = state
        .identity_repo
        .find_by_name(&auth.identity)?
        .ok_or(SealboxError::Unauthorized)?;
    if identity.role != Role::Admin {
        return Err(SealboxError::Forbidden);
    }

    let pending = state
        .passkey
        .take_approval(&params.id)
        .ok_or_else(|| SealboxError::InvalidRequest("that approval has expired".into()))?;

    let grant = super::grant::create_from_payload(&state, pending.payload, &auth.identity)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(auth.identity.clone()),
        action: "grant.approve".to_string(),
        resource: Some(grant.name.clone()),
        outcome: AuditOutcome::Allowed,
        detail: Some("approved with a passkey".to_string()),
    })?;

    Ok(SealboxResponse::Json(json!(grant)))
}

// ---------------------------------------------------------------- page

fn base64_id(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One page, built as a string. A templating engine would be a dependency to keep working
/// forever for a page that has one shape and — per ADR 0004 — must not grow another.
fn page(title: &str, body: &str, action: &str, base: &str, mode: &str) -> String {
    format!(
        r#"<!doctype html>
<meta charset=utf-8>
<meta name=viewport content="width=device-width,initial-scale=1">
<title>Sealbox — {title}</title>
<style>
  :root {{ color-scheme: light dark; }}
  body {{ font: 16px/1.55 system-ui, sans-serif; max-width: 34rem; margin: 3rem auto; padding: 0 1.25rem; }}
  h1 {{ font-size: 1rem; letter-spacing: .08em; text-transform: uppercase; opacity: .6; font-weight: 600; }}
  h2 {{ font-size: 1.5rem; margin: .2rem 0 1rem; }}
  code {{ font-family: ui-monospace, monospace; font-size: .9em; }}
  .lead {{ margin-bottom: .4rem; }}
  .secrets {{ margin: 0 0 1.2rem; padding-left: 1.2rem; }}
  .secrets li {{ margin: .2rem 0; }}
  .note {{ opacity: .65; font-size: .9em; }}
  button {{ font: inherit; padding: .6rem 1.4rem; border-radius: .4rem; border: 0;
            background: light-dark(#111, #eee); color: light-dark(#fff, #111); cursor: pointer; }}
  button[disabled] {{ opacity: .5; cursor: default; }}
  #result {{ margin-top: 1.2rem; }}
</style>
<h1>Sealbox</h1>
{body}
<p><label class=note>Identity <input id=identity value="" placeholder="admin"></label></p>
<p><button id=go>{action}</button></p>
<div id=result></div>
<script>
const b64u = {{
  dec: s => Uint8Array.from(atob(s.replace(/-/g,'+').replace(/_/g,'/')), c => c.charCodeAt(0)),
  enc: b => btoa(String.fromCharCode(...new Uint8Array(b))).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'')
}};
function say(text, ok) {{
  document.getElementById('result').innerHTML =
    '<p class=note style="color:' + (ok ? 'green' : 'crimson') + '">' + text + '</p>';
}}
document.getElementById('go').onclick = async () => {{
  const button = document.getElementById('go');
  button.disabled = true;
  try {{
    const identity = document.getElementById('identity').value.trim() || 'admin';
    const started = await fetch('{base}/start', {{
      method: 'POST', headers: {{'content-type': 'application/json'}},
      body: JSON.stringify({{ identity }})
    }});
    if (!started.ok) throw new Error(await started.text());
    const payload = await started.json();

    let credential;
    if ('{mode}' === 'register') {{
      const o = payload.publicKey;
      o.challenge = b64u.dec(o.challenge);
      o.user.id = b64u.dec(o.user.id);
      const c = await navigator.credentials.create({{ publicKey: o }});
      credential = {{
        id: c.id, rawId: b64u.enc(c.rawId), type: c.type,
        response: {{
          attestationObject: b64u.enc(c.response.attestationObject),
          clientDataJSON: b64u.enc(c.response.clientDataJSON)
        }}
      }};
    }} else {{
      const o = payload.options.publicKey;
      o.challenge = b64u.dec(o.challenge);
      (o.allowCredentials || []).forEach(c => c.id = b64u.dec(c.id));
      const c = await navigator.credentials.get({{ publicKey: o }});
      credential = {{
        id: c.id, rawId: b64u.enc(c.rawId), type: c.type,
        response: {{
          authenticatorData: b64u.enc(c.response.authenticatorData),
          clientDataJSON: b64u.enc(c.response.clientDataJSON),
          signature: b64u.enc(c.response.signature),
          userHandle: c.response.userHandle ? b64u.enc(c.response.userHandle) : null
        }}
      }};
    }}

    const body = '{mode}' === 'register'
      ? credential
      : {{ challenge_id: payload.challenge_id, credential }};
    const done = await fetch('{base}/finish', {{
      method: 'POST', headers: {{'content-type': 'application/json'}},
      body: JSON.stringify(body)
    }});
    if (!done.ok) throw new Error(await done.text());
    say('Done. You can close this page.', true);
  }} catch (e) {{
    say(String(e.message || e), false);
    button.disabled = false;
  }}
}};
</script>
"#
    )
}
