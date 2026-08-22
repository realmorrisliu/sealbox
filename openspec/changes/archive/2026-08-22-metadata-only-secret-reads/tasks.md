## 1. The read path

- [x] 1.1 `GET /v1/secrets/{key}` returns metadata only — key, version, master key id, timestamps, expiry
- [x] 1.2 No parameter, header, or role produces ciphertext; there is no tier to reach it through
- [x] 1.3 Confirm nothing else consumed ciphertext over HTTP (the runner receives plaintext; rekey is server-side)

## 2. Client

- [x] 2.1 Remove `secret get` and its client-side decryption
- [x] 2.2 Remove the `secret export`, `secret import`, and `secret history` stubs
- [x] 2.3 Make sure what remains still answers "does this exist, and when did it last change"

## 3. The cold path

- [x] 3.1 Record the offline tool's shape in the docs, and say plainly that it does not exist yet
- [x] 3.2 State the consequence where a cold secret is described: writable now, not readable back
      until that tool exists

## 4. Tests

- [x] 4.1 A read carries no ciphertext fields
- [x] 4.2 A read with a version carries no ciphertext fields either
- [x] 4.3 An agent still learns a secret exists and when it changed

## 5. Documentation

- [x] 5.1 Update the CLI reference: `secret get` is gone, and why
- [x] 5.2 Update the design document's secret lifecycle and honest-boundary sections
- [x] 5.3 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
