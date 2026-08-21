## 1. The grant model

- [ ] 1.1 Add a `Grant` struct: name, implementation, runner, declared secrets, chain, created_at, created_by
- [ ] 1.2 Model the implementation as an enum — a named adapter with its config, or a stored script — so "both" and "neither" are unrepresentable rather than validated
- [ ] 1.3 Define the known adapter names as a constant set (`kubernetes-secret`, `postgres-role`); recognising a name is all that happens here, the implementations come with the runner
- [ ] 1.4 Create the `grants` table with a unique index on the name
- [ ] 1.5 `GrantRepo`: create, get by name, list, remove, and list-by-declared-secret

## 2. Validation at creation

- [ ] 2.1 Refuse a grant declaring a secret that does not exist, naming the missing one
- [ ] 2.2 Refuse a grant naming an unrecognised adapter, naming the known ones
- [ ] 2.3 Refuse a chain naming a grant that does not exist
- [ ] 2.4 Refuse a chain that would revisit a grant already on the path — walk the graph rather than cap the depth, so the error describes the cycle instead of a depth limit
- [ ] 2.5 Refuse a duplicate name rather than replacing the existing grant

## 3. Endpoints

- [ ] 3.1 `POST /v1/grants` — admin only; creates and validates
- [ ] 3.2 `GET /v1/grants` and `GET /v1/grants/{name}` — any authenticated identity, so an agent can see what it may invoke
- [ ] 3.3 `DELETE /v1/grants/{name}` — admin only
- [ ] 3.4 `GET /v1/secrets?uses=<name>` or equivalent — every grant declaring that secret
- [ ] 3.5 Place each route in the correct role group; confirm an agent creating a grant gets 403, not 401

## 4. Client

- [ ] 4.1 `sealbox-cli grant add <file>` — parse TOML locally so a malformed file fails with the file in hand
- [ ] 4.2 `sealbox-cli grant list` / `grant show <name>` / `grant rm <name>`
- [ ] 4.3 `sealbox-cli secret uses <name>` — the grants that may use a secret
- [ ] 4.4 On `grant add`, print the declaration being approved — the secrets above all — so the reviewer sees what actually matters

## 5. Tests

- [ ] 5.1 A grant round-trips: created, shown, listed, removed
- [ ] 5.2 An agent creating a grant is refused with 403; listing succeeds
- [ ] 5.3 A grant declaring a missing secret is refused, and the error names it
- [ ] 5.4 An unknown adapter is refused at creation
- [ ] 5.5 Both an adapter and a script is refused; neither is refused
- [ ] 5.6 A chain to a missing grant is refused
- [ ] 5.7 A cycle is refused, including one formed indirectly through a third grant
- [ ] 5.8 A duplicate name is refused and the original survives unchanged
- [ ] 5.9 `--uses` returns exactly the grants declaring a secret, and an empty result for one nothing uses
- [ ] 5.10 A script is stored with the grant and returned when shown

## 6. Documentation

- [ ] 6.1 Update `docs/cli-reference.md` for the grant commands and the grant file format
- [ ] 6.2 Update `examples/grants/` if the shipped examples differ from what is accepted
- [ ] 6.3 Update `CLAUDE.md`: MVP item 4 partly done — definition and approval yes, execution and adapters no
- [ ] 6.4 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
