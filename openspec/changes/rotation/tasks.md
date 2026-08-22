## 1. Pending versions

- [ ] 1.1 Add `pending` to the `secrets` table, defaulting to 0 so every existing row is current
- [ ] 1.2 Exclude pending versions from every read path: get, get-by-version, list, and the claim's secret lookup
- [ ] 1.3 `SecretRepo`: create a pending version, commit one, discard one
- [ ] 1.4 Confirm a pending version is invisible even to a grant that declares the secret

## 2. Rotation

- [ ] 2.1 Add a rotation to the job model: which secret, and whether the value is captured
- [ ] 2.2 `POST /v1/secrets/{key}/rotate` — operator and above; generates the value, writes it pending, queues the job
- [ ] 2.3 Reject a rotation request carrying a value: the system generates it, never the caller
- [ ] 2.4 Add the generated value to the claim's secrets as `SEALBOX_NEW`, so a runner materialises it exactly like any other and an implementation cannot tell it apart
- [ ] 2.5 On success, commit the pending version; on failure, discard it
- [ ] 2.6 Discard the pending version when the sweeper fails an abandoned rotation

## 3. Capture

- [ ] 3.1 Extend the report payload with a captured value, separate from output
- [ ] 3.2 Store a captured value into the pending version; **never** into the job record
- [ ] 3.3 Fail the rotation when capture is requested and nothing was emitted — storing an empty credential is the same failure as storing the wrong one
- [ ] 3.4 In the runner, take stdout as the captured value when the claim asks for it, and leave stderr as output

## 4. Client

- [ ] 4.1 `sealbox-cli rotate <secret> --via <grant> [--from-output] [key=value ...]`
- [ ] 4.2 Report the outcome plainly: which value is now current, or that the previous one still is
- [ ] 4.3 Say that the new value was never displayed, since a caller may otherwise wait for it

## 5. Tests

- [ ] 5.1 A successful rotation makes the new value current
- [ ] 5.2 A failed rotation leaves the previous value current, byte for byte
- [ ] 5.3 A pending version is invisible to reads, listings, and a claim
- [ ] 5.4 The implementation receives the generated value indistinguishably from a declared secret
- [ ] 5.5 A rotation request carrying a value is rejected
- [ ] 5.6 A captured value becomes current and appears nowhere in the job record
- [ ] 5.7 Capturing nothing fails the rotation and leaves the previous value
- [ ] 5.8 An abandoned rotation discards its pending version
- [ ] 5.9 An agent cannot rotate; an operator can

## 6. Documentation

- [ ] 6.1 Update `docs/cli-reference.md` for `rotate` and for what a capturing grant must print
- [ ] 6.2 Update `examples/grants/rotate-db.toml` to match what is accepted
- [ ] 6.3 Update `CLAUDE.md`: MVP item 7 done
- [ ] 6.4 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
