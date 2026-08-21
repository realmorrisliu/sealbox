## 1. Server-side generation

- [x] 1.1 Add a `GenerateSpec` type: kind (`password` | `hex`) and an optional length
- [x] 1.2 Make the save payload accept either a supplied value or a generation request, mutually exclusive, rejecting unknown fields
- [x] 1.3 Generate the value inside the repository, next to `Secret::new`, so the plaintext exists only for the duration of that call — never assigned to a field, returned, or logged
- [x] 1.4 Draw `password` from an alphabet excluding `0`/`O` and `1`/`l`/`I`; leave `hex` as raw bytes
- [x] 1.5 Apply defaults (32) and enforce the minimum (16), refusing a shorter length with an error that names the minimum
- [x] 1.6 Confirm the response reports only key, version, and timestamps. It was returning the whole `Secret` — ciphertext, encrypted data key, and master key id — which handed every caller the material to decrypt with, given a master key, for no reason

## 2. Client

- [ ] 2.1 `secret set <key>` reads the value from stdin only; remove the positional argument
- [ ] 2.2 Fail with a message naming stdin when nothing is piped and the input is not a terminal
- [ ] 2.3 Add `secret gen <key> --type password|hex [--length N] [--ttl N]`
- [ ] 2.4 Fix `secret list` to call `GET /v1/secrets` and print what it returns, instead of claiming the server cannot list
- [ ] 2.5 Confirm no command accepts a secret's value as an argument

## 3. Tests

- [x] 3.1 A generated secret is stored, and the response contains no value
- [x] 3.2 Two generations with identical parameters produce different values
- [x] 3.3 A length below the minimum is refused, and the error names the minimum
- [x] 3.4 A payload supplying both a value and a generation request is rejected
- [ ] 3.5 Generating over an existing key creates a new version, leaving the previous retrievable
- [x] 3.6 Listing returns metadata with no value, ciphertext, or encrypted data key in any entry
- [ ] 3.7 An expired secret does not appear in a listing
- [ ] 3.8 The generated password alphabet excludes the ambiguous characters

## 4. Documentation

- [ ] 4.1 Update `docs/cli-reference.md` for `secret set` reading stdin and the new `secret gen`
- [ ] 4.2 Update `docs/getting-started.md` where it shows storing a value
- [ ] 4.3 Update `CLAUDE.md`: MVP item 3 done
- [ ] 4.4 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
