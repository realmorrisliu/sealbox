## 1. Issuers

- [ ] 1.1 Store issuers: name, issuer URL, and one or more keys
- [ ] 1.2 Admin-only registration, update, and removal
- [ ] 1.3 More than one key at a time, so a rotation overlaps rather than cuts over

## 2. Binding a runner

- [ ] 2.1 A runner identity may carry an issuer, a subject, and an expected audience instead of a token hash
- [ ] 2.2 Exact subject matching — no prefixes, no patterns
- [ ] 2.3 An identity is bound to an issuer **or** holds a token, never both

## 3. Verification

- [ ] 3.1 Verify the signature against the issuer's registered keys, offline
- [ ] 3.2 Check `exp`, `nbf`, and `iat` with a bounded clock skew
- [ ] 3.3 Require the expected audience
- [ ] 3.4 Refuse a revoked identity regardless of the token
- [ ] 3.5 Audit each refusal with the step that failed

## 4. The runner

- [ ] 4.1 Read the token from a file and re-read it before each claim — the platform rotates it underneath
- [ ] 4.2 Fail loudly and specifically when the file is missing, unreadable, or not a JWT
- [ ] 4.3 Keep the bearer-token path working, since agents and operators still use it

## 5. Tests

- [ ] 5.1 A valid token authenticates
- [ ] 5.2 Wrong subject, wrong audience, expired, and unknown key are each refused
- [ ] 5.3 A subject differing only by a suffix is refused
- [ ] 5.4 A revoked identity is refused a valid token
- [ ] 5.5 Both keys work during a rotation
- [ ] 5.6 Nothing reaches the network during verification

## 6. Documentation

- [ ] 6.1 The Deployment manifest, including the projected token with a non-default audience
- [ ] 6.2 How to get a cluster's JWKS, and what to do when it rotates
- [ ] 6.3 Correct `docs/agent-native-design.md`, which still specifies join tokens, and say why they were not built

## 7. Verification

- [ ] 7.1 `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
- [ ] 7.2 End to end against the local cluster: a runner in a pod, authenticating with its ServiceAccount token, claims and executes a job
