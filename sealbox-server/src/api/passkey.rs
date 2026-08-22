//! Admin authentication by passkey, and the approval ceremony it exists for.
//!
//! Two problems are being solved, and only the first is about credentials.
//!
//! A token in a file can be read by an agent on the same machine, so a passkey's private half
//! stays in a Secure Enclave and nothing on disk is worth stealing. But a better credential does
//! nothing about the second: **a terminal cannot be a trusted display**, because its output is
//! written by whatever process is running. An agent can print one grant's declaration and submit
//! another. So what is approved is rendered by the server and stored server-side — never sent by
//! the caller — and the signature is bound to what was stored (ADR 0009).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::error::{Result, SealboxError};

/// How long a challenge stays valid. Long enough to reach for a phone, short enough that a
/// captured one is of little use.
pub const CHALLENGE_TTL: Duration = Duration::from_secs(5 * 60);
/// How long one authentication covers. Long enough that importing fifty credentials is one
/// prompt — intolerable security gets bypassed, which is a design constraint, not a nicety.
pub const SESSION_TTL: Duration = Duration::from_secs(10 * 60);
/// How long an enrolment link lives.
pub const ENROLMENT_TTL: Duration = Duration::from_secs(30 * 60);

/// Everything here is deliberately in memory.
///
/// A persisted session would be a credential at rest — the thing this change removes — with the
/// added irony of storing it in the database it protects. Losing them on restart costs one extra
/// prompt.
#[derive(Clone)]
pub(crate) struct PasskeyState {
    pub webauthn: Arc<Webauthn>,
    registrations: Arc<Mutex<HashMap<Uuid, Pending<PasskeyRegistration>>>>,
    authentications: Arc<Mutex<HashMap<Uuid, Pending<AuthState>>>>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    enrolments: Arc<Mutex<HashMap<Uuid, Pending<String>>>>,
    approvals: Arc<Mutex<HashMap<Uuid, Pending<PendingApproval>>>>,
    /// Login requests a CLI is waiting on: empty until a browser finishes the ceremony, then
    /// holding the session for one collection.
    logins: Arc<Mutex<HashMap<Uuid, Pending<Option<String>>>>>,
}

struct Pending<T> {
    value: T,
    expires: Instant,
}

impl<T> Pending<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires: Instant::now() + ttl,
        }
    }
    fn alive(&self) -> bool {
        Instant::now() < self.expires
    }
}

pub(crate) struct AuthState {
    pub identity: String,
    pub state: PasskeyAuthentication,
    /// When this authentication is for approving something, which one. Bound here rather than
    /// travelling with the request: otherwise an agent could show one thing and sign another,
    /// which is the attack the rendered page exists to prevent.
    pub approval: Option<Uuid>,
}

pub(crate) struct Session {
    pub identity: String,
    expires: Instant,
}

/// A grant awaiting approval. Stored server-side so the page renders from it, and so the
/// signature can be bound to it.
#[derive(Clone, Debug)]
pub(crate) struct PendingApproval {
    pub payload: serde_json::Value,
    pub requested_by: String,
}

impl PasskeyState {
    /// The relying party is the server's public URL. Every registration is bound to it, so
    /// changing the hostname invalidates every passkey — which is why a missing value is fatal
    /// rather than defaulted.
    pub fn new(public_url: &str) -> Result<Self> {
        let origin = Url::parse(public_url).map_err(|e| {
            SealboxError::ConfigError(format!(
                "SEALBOX_PUBLIC_URL is not a URL: {e}. It is the WebAuthn relying-party ID, so \
                 every registered passkey is bound to it."
            ))
        })?;
        let rp_id = origin
            .host_str()
            .ok_or_else(|| SealboxError::ConfigError("SEALBOX_PUBLIC_URL has no host".to_string()))?
            .to_string();

        let webauthn = WebauthnBuilder::new(&rp_id, &origin)
            .and_then(|b| b.rp_name("Sealbox").build())
            .map_err(|e| SealboxError::ConfigError(format!("WebAuthn setup failed: {e}")))?;

        Ok(Self {
            webauthn: Arc::new(webauthn),
            registrations: Default::default(),
            authentications: Default::default(),
            sessions: Default::default(),
            enrolments: Default::default(),
            approvals: Default::default(),
            logins: Default::default(),
        })
    }

    // ---- enrolment -------------------------------------------------------

    /// A link that works once, expires, and — enforced by the caller — only for an identity with
    /// no authenticator yet. A leaked link must be a way to become *an* admin for the first
    /// time, never a way to displace a working credential.
    pub fn issue_enrolment(&self, identity: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.enrolments
            .lock()
            .unwrap()
            .insert(id, Pending::new(identity.to_string(), ENROLMENT_TTL));
        id
    }

    pub fn peek_enrolment(&self, id: &Uuid) -> Option<String> {
        let map = self.enrolments.lock().unwrap();
        map.get(id).filter(|p| p.alive()).map(|p| p.value.clone())
    }

    /// Consuming is separate from peeking so the page can be rendered without spending the link.
    pub fn consume_enrolment(&self, id: &Uuid) -> Option<String> {
        let mut map = self.enrolments.lock().unwrap();
        map.remove(id).filter(|p| p.alive()).map(|p| p.value)
    }

    // ---- registration ----------------------------------------------------

    pub fn stash_registration(&self, id: Uuid, state: PasskeyRegistration) {
        self.registrations
            .lock()
            .unwrap()
            .insert(id, Pending::new(state, CHALLENGE_TTL));
    }

    pub fn take_registration(&self, id: &Uuid) -> Option<PasskeyRegistration> {
        let mut map = self.registrations.lock().unwrap();
        map.remove(id).filter(|p| p.alive()).map(|p| p.value)
    }

    // ---- authentication --------------------------------------------------

    pub fn stash_authentication(&self, id: Uuid, state: AuthState) {
        self.authentications
            .lock()
            .unwrap()
            .insert(id, Pending::new(state, CHALLENGE_TTL));
    }

    /// Single use: taking it removes it, so a replayed signature finds nothing.
    pub fn take_authentication(&self, id: &Uuid) -> Option<AuthState> {
        let mut map = self.authentications.lock().unwrap();
        map.remove(id).filter(|p| p.alive()).map(|p| p.value)
    }

    // ---- sessions --------------------------------------------------------

    pub fn issue_session(&self, identity: &str) -> String {
        let token = format!("sealbox_session_{}", Uuid::new_v4().simple());
        self.sessions.lock().unwrap().insert(
            token.clone(),
            Session {
                identity: identity.to_string(),
                expires: Instant::now() + SESSION_TTL,
            },
        );
        token
    }

    pub fn resolve_session(&self, token: &str) -> Option<String> {
        let map = self.sessions.lock().unwrap();
        map.get(token)
            .filter(|s| Instant::now() < s.expires)
            .map(|s| s.identity.clone())
    }

    pub fn sweep(&self) {
        self.registrations.lock().unwrap().retain(|_, p| p.alive());
        self.authentications
            .lock()
            .unwrap()
            .retain(|_, p| p.alive());
        self.enrolments.lock().unwrap().retain(|_, p| p.alive());
        self.approvals.lock().unwrap().retain(|_, p| p.alive());
        self.logins.lock().unwrap().retain(|_, p| p.alive());
        self.sessions
            .lock()
            .unwrap()
            .retain(|_, s| Instant::now() < s.expires);
    }

    // ---- approvals -------------------------------------------------------

    pub fn stash_approval(&self, approval: PendingApproval) -> Uuid {
        let id = Uuid::new_v4();
        self.approvals
            .lock()
            .unwrap()
            .insert(id, Pending::new(approval, CHALLENGE_TTL));
        id
    }

    pub fn peek_approval(&self, id: &Uuid) -> Option<PendingApproval> {
        let map = self.approvals.lock().unwrap();
        map.get(id).filter(|p| p.alive()).map(|p| p.value.clone())
    }

    pub fn take_approval(&self, id: &Uuid) -> Option<PendingApproval> {
        let mut map = self.approvals.lock().unwrap();
        map.remove(id).filter(|p| p.alive()).map(|p| p.value)
    }

    // ---- logins ----------------------------------------------------------

    /// A waiting CLI. The session travels through here rather than being shown on the page and
    /// typed back: a credential a human copies is a credential that lands in scrollback.
    pub fn open_login(&self) -> Uuid {
        let id = Uuid::new_v4();
        self.logins
            .lock()
            .unwrap()
            .insert(id, Pending::new(None, CHALLENGE_TTL));
        id
    }

    pub fn login_is_open(&self, id: &Uuid) -> bool {
        let map = self.logins.lock().unwrap();
        map.get(id).is_some_and(|p| p.alive())
    }

    pub fn complete_login(&self, id: &Uuid, session: String) -> bool {
        let mut map = self.logins.lock().unwrap();
        match map.get_mut(id).filter(|p| p.alive()) {
            Some(pending) => {
                pending.value = Some(session);
                true
            }
            None => false,
        }
    }

    /// Collected once: the waiting CLI takes it, and a second poll — or anyone else who guesses
    /// the id afterwards — finds nothing.
    pub fn collect_login(&self, id: &Uuid) -> Option<String> {
        let mut map = self.logins.lock().unwrap();
        let session = map.get_mut(id).filter(|p| p.alive())?.value.take();
        if session.is_some() {
            map.remove(id);
        }
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PasskeyState {
        PasskeyState::new("https://sealbox.example.dev").expect("Should build")
    }

    #[test]
    fn a_session_resolves_then_expires() {
        let s = state();
        let token = s.issue_session("root");
        assert_eq!(s.resolve_session(&token).as_deref(), Some("root"));

        // Forcibly expire it, standing in for the passage of time.
        s.sessions.lock().unwrap().get_mut(&token).unwrap().expires = Instant::now();
        assert!(
            s.resolve_session(&token).is_none(),
            "an expired session must not authenticate"
        );
    }

    #[test]
    fn an_unknown_session_resolves_to_nobody() {
        assert!(state().resolve_session("sealbox_session_made-up").is_none());
    }

    #[test]
    fn an_enrolment_link_works_once() {
        let s = state();
        let id = s.issue_enrolment("root");

        // Peeking may happen repeatedly — rendering the page must not spend the link.
        assert_eq!(s.peek_enrolment(&id).as_deref(), Some("root"));
        assert_eq!(s.peek_enrolment(&id).as_deref(), Some("root"));

        assert_eq!(s.consume_enrolment(&id).as_deref(), Some("root"));
        assert!(
            s.consume_enrolment(&id).is_none(),
            "a second use must find nothing"
        );
    }

    #[test]
    fn an_approval_is_taken_once() {
        let s = state();
        let id = s.stash_approval(PendingApproval {
            payload: serde_json::json!({ "name": "k8s-sync" }),
            requested_by: "root".to_string(),
        });

        assert!(s.peek_approval(&id).is_some());
        assert!(s.take_approval(&id).is_some());
        assert!(
            s.take_approval(&id).is_none(),
            "an approval cannot be replayed into a second grant"
        );
    }

    #[test]
    fn sweeping_removes_what_has_expired() {
        let s = state();
        let live = s.issue_enrolment("root");
        let stale = s.issue_enrolment("other");
        s.enrolments
            .lock()
            .unwrap()
            .get_mut(&stale)
            .unwrap()
            .expires = Instant::now();

        s.sweep();

        assert!(s.peek_enrolment(&live).is_some());
        assert!(s.peek_enrolment(&stale).is_none());
    }

    #[test]
    fn a_session_reaches_a_waiting_cli_exactly_once() {
        let s = state();
        let id = s.open_login();
        assert!(s.collect_login(&id).is_none(), "nothing until it is signed");

        assert!(s.complete_login(&id, "sealbox_session_x".to_string()));
        assert_eq!(s.collect_login(&id).as_deref(), Some("sealbox_session_x"));
        assert!(
            s.collect_login(&id).is_none(),
            "a second poll must not hand the session to someone else"
        );
    }

    #[test]
    fn a_challenge_is_single_use() {
        // The registration and authentication maps behave identically; either one demonstrates
        // it, and `take_*` is the only way in, so a replay finds nothing.
        let s = state();
        let id = Uuid::new_v4();
        s.approvals
            .lock()
            .unwrap()
            .insert(id, Pending::new(approval(), CHALLENGE_TTL));

        assert!(s.take_approval(&id).is_some());
        assert!(
            s.take_approval(&id).is_none(),
            "a replayed challenge must find nothing"
        );
    }

    #[test]
    fn an_expired_challenge_is_refused() {
        let s = state();
        let id = Uuid::new_v4();
        s.approvals
            .lock()
            .unwrap()
            .insert(id, Pending::new(approval(), Duration::ZERO));

        assert!(
            s.take_approval(&id).is_none(),
            "expiry is checked on the way out, so a challenge that outlived its TTL cannot be              used even though the entry is still in the map"
        );
    }

    fn approval() -> PendingApproval {
        PendingApproval {
            payload: serde_json::json!({ "name": "k8s-sync" }),
            requested_by: "root".to_string(),
        }
    }

    #[test]
    fn the_relying_party_comes_from_the_public_url() {
        assert!(PasskeyState::new("not a url").is_err());
        assert!(PasskeyState::new("https://sealbox.example.dev").is_ok());
    }
}
