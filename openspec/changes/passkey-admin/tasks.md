## 1. Registered authenticators

- [ ] 1.1 Add a table of registered authenticators per identity, storing only what WebAuthn needs to *verify* — never anything that could produce a valid authentication
- [ ] 1.2 `AuthenticatorRepo`: register, list for an identity, and count
- [ ] 1.3 Configure the relying party from `SEALBOX_PUBLIC_URL`; fail startup with a clear message when it is missing, since every registration is bound to it

## 2. Enrolment

- [ ] 2.1 Single-use, expiring enrolment links, valid only for an identity with no authenticator — a leaked link must be a way to become *an* admin for the first time, never a way to displace a working one
- [ ] 2.2 `bootstrap` returns an enrolment link instead of a token
- [ ] 2.3 `identity create --role admin` returns an enrolment link and no credential
- [ ] 2.4 Serve the enrolment page and handle the registration response

## 3. Authentication and sessions

- [ ] 3.1 Issue single-use challenges with a bounded lifetime; refuse a replayed or expired one
- [ ] 3.2 Verify the signature and issue a session
- [ ] 3.3 Hold sessions in memory with an expiry — persisting one would make it a credential at rest, which is what this change removes
- [ ] 3.4 Sweep expired sessions

## 4. The gate

- [ ] 4.1 Admin routes accept a session and **refuse a bearer token outright**, even a valid admin identity's — a route that still accepts one leaves the hole open for anything that forgets to stop sending it
- [ ] 4.2 Leave agent, operator, and runner authentication untouched
- [ ] 4.3 Confirm no admin route can be reached with any bearer token

## 5. Approval

- [ ] 5.1 `grant add` uploads the grant as a *pending approval* and returns an id; the grant is created only when the approval is signed
- [ ] 5.2 Render the approval page from what the server stored — never from anything the caller sends, or an agent could show one thing and sign another
- [ ] 5.3 Show the declared secrets first; that line is what the approval is actually about
- [ ] 5.4 Bind the signature to the approval id and refuse a mismatched subject
- [ ] 5.5 Expire pending approvals

## 6. Client

- [ ] 6.1 `sealbox-cli admin` authenticates once and holds the session in process memory
- [ ] 6.2 `grant add` opens the approval URL and polls; print the URL as well, so it can be opened on a phone
- [ ] 6.3 Say plainly that approving on another device is supported, and why it is better

## 7. Tests

- [ ] 7.1 A challenge is single-use; a replay is refused
- [ ] 7.2 An expired challenge is refused
- [ ] 7.3 An approval signed for a different subject is refused
- [ ] 7.4 A session expires, and a fresh one is required
- [ ] 7.5 An admin route refuses a bearer token, including a valid admin identity's
- [ ] 7.6 Agent, operator, and runner tokens still work everywhere they did
- [ ] 7.7 An enrolment link is single-use, expires, and is refused for an identity that already has an authenticator
- [ ] 7.8 Stored authenticator data cannot be replayed to authenticate

## 8. Documentation

- [ ] 8.1 Document the admin flow in `docs/cli-reference.md` and `docs/getting-started.md`
- [ ] 8.2 State in `docs/configuration.md` that `SEALBOX_PUBLIC_URL` is the relying-party ID and that changing it invalidates every passkey
- [ ] 8.3 Update `CLAUDE.md`: MVP item 9 done, and the security boundary's remaining hole closed
- [ ] 8.4 Say explicitly what could not be verified here: the browser and biometric parts need a person
- [ ] 8.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
