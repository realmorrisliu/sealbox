## Context

See proposal.md — Why.

Admin identities hold a bearer token like every other role. Approvals happen in a terminal, whose
output is written by whatever process is running.

## Goals / Non-Goals

**Goals:**

- Nothing on an admin's machine authenticates as that admin.
- What a human signs is what the server rendered, not what a caller showed them.
- One authentication covers a burst of work, so the mechanism is not bypassed.

**Non-Goals:**

- Passkeys for agents, operators, or runners. They have no fingers; a bearer token bounded by
  role is right for them.
- Account recovery through a second factor. Losing every authenticator is survivable because the
  recovery key decrypts the database independently of authentication (ADR 0010) — authentication
  and encryption fail independently, by design.
- Anything else on the approval page. It is an authentication ceremony (ADR 0004); the moment it
  grows a secret list it has become the web UI that decision rejected.

## Decisions

### The browser does the WebAuthn, and the CLI waits

The CLI opens `/{approve,enrol,authenticate}/<id>` and polls. It does not speak CTAP.

Direct CTAP reaches only hardware keys — Touch ID and Windows Hello are not exposed over CTAP-HID
— and, more importantly, a terminal has no trusted display. Going through the browser is what
makes the second half of this feature work at all, not a convenience.

It also makes cross-device approval free: the URL can be opened on a phone, which is a device the
agent has no access to.

### Sessions live in a map in the server, not in a table

A session is an identity, an expiry, and nothing else, held in memory. Restarting the server ends
every session.

Persisting them would make a session a credential at rest — the thing this change exists to
remove — with the added irony of storing it in the database it protects. Losing sessions on
restart is a cost measured in one extra prompt.

### The approval subject is stored server-side and never sent by the caller

`grant add` uploads the grant, the server stores it as a *pending approval* and returns an id.
The page renders from what the server stored. The signature covers that id.

If the subject travelled with the approval, an agent could show one thing and sign another — the
exact attack the rendered page exists to prevent, reintroduced one layer down.

### Admin operations take a session token, and bearer tokens are refused outright

Not "a session or a token, whichever is present". An admin route rejects a bearer token even if
it belongs to a valid admin identity, because a route that still accepts one leaves the hole open
for anything that forgets to stop sending it.

Admin identities therefore have no bearer token at all. `bootstrap` returns an enrolment link.

### Enrolment cannot displace an existing authenticator

An enrolment link works only for an identity with none registered. Otherwise a leaked link would
be a way to become the admin, rather than a way to become *an* admin for the first time.

Rotating an authenticator is therefore: create a new admin identity, enrol it, revoke the old one
— all of which are already audited operations.

### The page is a string, not a template

One page, built with `format!`, with values escaped where they are interpolated. A templating
engine would be a dependency to keep working forever for a page that has one shape and will not
grow — because ADR 0004 says it must not.

## Risks / Trade-offs

- **WebAuthn needs a secure context** → HTTPS in anything but local development. Fly.io terminates
  TLS; `localhost` is exempt, so development works unchanged.
- **The public URL is the relying-party ID** → Changing the hostname invalidates every registered
  passkey. Documented, and recoverable: the recovery key does not depend on authentication.
- **Losing every authenticator locks out administration** → Recovery is the recovery key, which
  decrypts the database directly. Enrolling a second admin identity is the cheap precaution.
- **The browser flow cannot be tested end to end here** → Server-side logic — challenge issue,
  replay, expiry, subject binding, session lifetime — is testable and tested. The part that needs
  a finger needs a person; that is stated rather than papered over.
- **A session is a bearer credential for its lifetime** → In memory, short, and never written
  down. An agent could read it out of a live process, which is the residual risk already recorded
  in the design's security boundary, and approving from a phone avoids even that.

## Migration Plan

Existing admin identities have a bearer token and no authenticator, so they stop being able to
perform admin operations. Under ADR 0012 that is acceptable: re-bootstrap, or issue an enrolment
link from an identity that still works.

**Rollback:** revert the binary. Registered authenticators become inert rows; admin bearer tokens
work again.

## Open Questions

None.
