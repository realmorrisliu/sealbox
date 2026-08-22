## Why

Everything built so far confines what an agent can do. One thing is still open, and it undoes all
of it: **an agent sharing a machine with an admin can read that admin's token and approve its own
grants** ([ADR 0009](../../../docs/adr/0009-admin-authenticates-with-passkeys.md)).

That is not a small hole. The whole design rests on a human deciding what is possible while the
agent decides only when — and a token in a file makes the first half a formality.

There is a second problem, less obvious and not fixed by a better credential. **A terminal cannot
be a trusted display.** Its output is written by whatever process is running, so an agent can
print one grant's declaration and submit another. Any approval that a human confirms by reading
their own terminal is confirming something an agent chose to show them.

A passkey answers both. Its private half lives in a Secure Enclave or a security key and is
unusable without the human present, so there is nothing on disk worth stealing. And the approval
page is **rendered by the server**, which an agent cannot influence — so the declaration a human
reads is the one they sign. It can also be read on a phone, which puts the approval on a device
the agent has no access to at all.

## What Changes

- **BREAKING** — an admin identity no longer authenticates with a bearer token. It registers a
  passkey, and admin operations require a session obtained by using it.
- Add WebAuthn registration and authentication, with the server's public URL as the relying party.
- Add an **approval page**: one server-rendered page showing what is being approved — the declared
  secrets above all — signed with a passkey.
- Add `sealbox admin`, which authenticates once and holds a short-lived session **in process
  memory, never written to disk**, so importing fifty credentials is one prompt rather than fifty.
  Intolerable security gets bypassed; that is a design constraint, not a nicety.
- `bootstrap` returns a registration link instead of a token.
- Agent, operator, and runner identities are unchanged — they hold bearer tokens, because they
  have no fingers.

## Capabilities

### New Capabilities

- `admin-approval`: how a human proves they are an admin, what makes an approval trustworthy, and
  what a session is allowed to be.

### Modified Capabilities

- `identity`: admin identities authenticate differently from every other role, and the
  requirement that credentials are stored as hashes needs to say what happens when there is no
  credential to store.

## Impact

**Completes** MVP item 9, and closes the last hole named in the design's security boundary.

**Constrained by** ADR 0009 (this is its implementation), ADR 0004 (the approval page is an
authentication ceremony, not an interface — one server-rendered page, no session storage, no
stored token, no SPA, and it must never grow a way to *manage* anything), and ADR 0003 (approving
a capability is the act this protects).

**Code**
- `sealbox-server/src/api/` — WebAuthn registration and authentication, sessions, the approval page
- `sealbox-server/src/repo/` — registered credentials per identity, pending approvals
- `sealbox-cli/src/` — `admin`, and opening a browser during `grant add`

**Dependencies** — `webauthn-rs`, and a template-free HTML page built as a string. A templating
engine for one page is a dependency to maintain forever.

**Operational** — WebAuthn requires a secure context, so the server needs HTTPS in anything but
local development, and its public URL becomes the relying-party ID: **changing the hostname
invalidates every registered passkey**. Recovery does not depend on it — the recovery key
decrypts the database independently of authentication — but it is not something to discover
during an incident.
