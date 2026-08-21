# Admin authenticates with passkeys, not a stored token

Every admin operation is authorised by a passkey (WebAuthn) challenge-response. **No admin
credential is stored on any machine.** Agent and runner identities keep bearer tokens.

This closes the last architectural hole in the design. Everything else confines what an agent can
do, but an agent sharing a machine with a human could read that human's admin token and approve
its own grants — collapsing the whole model. A passkey's private half lives in a Secure Enclave
or a security key and is unusable without the human present, so there is nothing on disk worth
stealing.

## Through a browser, not CTAP

The CLI does not talk to authenticators directly. `sealbox grant add` uploads the content, the
server records a pending approval, and the CLI opens
`https://<server>/approve/<id>`. The page shows what is being approved; Touch ID signs it.

Direct CTAP was rejected: it reaches only hardware keys — platform authenticators such as Touch
ID and Windows Hello are not exposed over CTAP-HID — and a terminal has no trusted display.

That trusted display is the second reason for this decision, not a side effect. "What you see is
not what you sign" is otherwise undefeatable: terminal output is controlled by whatever process
is running, so an agent could show one grant and submit another. **The approval page is rendered
by the server**, which an agent cannot influence, so the capability declaration a human reads is
the one they sign.

Third: approval can happen on a **different device**. An agent drafts a grant on a laptop; the
human approves it on a phone. The operational rule "approve where agents cannot reach" becomes
the default behaviour rather than a discipline.

## Not a web UI

ADR 0004 stands: there is no management interface. The approval page is part of an
authentication ceremony — one server-rendered page plus WebAuthn JavaScript, with no session
storage, no stored token, no SPA, and no CORS. It exists solely to supply the trusted display a
CLI cannot.

## Sessions, so that bulk work stays possible

Requiring a fingerprint per operation would make importing dozens of existing credentials
intolerable, and intolerable security gets bypassed. After one passkey authentication, `sealbox
admin` holds a short-lived session **in process memory, never written to disk**:

```
sealbox admin
> set utopia/prod/database-url
> grant add ./k8s-sync.toml
> exit                              # session dies with the process
```

Nothing usable remains on the filesystem afterwards. A same-uid agent could still ptrace a live
CLI process — deliberate attack, not the accidental leakage and post-injection abuse this design
targets.

## Consequences

Bootstrapping needs care: registering the first passkey happens before any admin credential
exists, so it is a one-time flow on first run — whoever reaches a fresh server first claims it.

Recovery needs a second factor that is not a passkey: losing every registered authenticator must
not mean losing the store. The offline master-key backup already required by ADR 0001 serves this, since
it can decrypt the database directly.
