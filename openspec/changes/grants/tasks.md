## 1. The grant model

- [x] 1.1 Add a `Grant` struct: name, implementation, runner, declared secrets, chain, created_at, created_by
- [x] 1.2 Model the implementation as an enum — a named adapter with its config, or a stored script — so "both" and "neither" are unrepresentable rather than validated
- [x] 1.3 Define the known adapter names as a constant set (`kubernetes-secret`, `postgres-role`); recognising a name is all that happens here, the implementations come with the runner
- [x] 1.4 Create the `grants` table with a unique index on the name
- [x] 1.5 `GrantRepo`: create, get by name, list, remove, and list-by-declared-secret

## 2. Validation at creation

- [x] 2.1 Refuse a grant declaring a secret that does not exist, naming the missing one. Also refuse a **parameterised** secret name — found end-to-end: the examples used `utopia/{env}/database-url`, but the parameter comes from whoever invokes the grant, so it would let them choose which credential it reaches
- [x] 2.2 Refuse a grant naming an unrecognised adapter, naming the known ones
- [x] 2.3 Refuse a chain naming a grant that does not exist
- [x] 2.4 Refuse a chain that would revisit a grant already on the path — walk the graph rather than cap the depth, so the error describes the cycle instead of a depth limit
- [x] 2.5 Refuse a duplicate name rather than replacing the existing grant

## 3. Endpoints

- [x] 3.1 `POST /v1/grants` — admin only; creates and validates
- [x] 3.2 `GET /v1/grants` and `GET /v1/grants/{name}` — any authenticated identity, so an agent can see what it may invoke
- [x] 3.3 `DELETE /v1/grants/{name}` — admin only
- [x] 3.4 `GET /v1/secrets?uses=<name>` or equivalent — every grant declaring that secret
- [x] 3.5 Place each route in the correct role group; confirm an agent creating a grant gets 403, not 401

## 4. Client

- [x] 4.1 `sealbox-cli grant add <file>` — parse TOML locally so a malformed file fails with the file in hand
- [x] 4.2 `sealbox-cli grant list` / `grant show <name>` / `grant rm <name>`
- [x] 4.3 `sealbox-cli secret uses <name>` — the grants that may use a secret
- [x] 4.4 On `grant add`, print the declaration being approved — the secrets above all — so the reviewer sees what actually matters

## 5. Tests

- [x] 5.1 A grant round-trips: created, shown, listed, removed
- [x] 5.2 An agent creating a grant is refused with 403; listing succeeds
- [x] 5.3 A grant declaring a missing secret is refused, and the error names it
- [x] 5.4 An unknown adapter is refused at creation
- [x] 5.5 Both an adapter and a script is refused; neither is refused
- [x] 5.6 A chain to a missing grant is refused
- [x] 5.7 A cycle is refused, including one formed indirectly through a third grant
- [x] 5.8 A duplicate name is refused and the original survives unchanged
- [x] 5.9 `--uses` returns exactly the grants declaring a secret, and an empty result for one nothing uses
- [x] 5.10 A script is stored with the grant and returned when shown

## 6. Documentation

- [x] 6.1 Update `docs/cli-reference.md` for the grant commands and the grant file format
- [x] 6.2 Update `examples/grants/` — they used parameterised secret names, which are now refused. Corrected there and in the design document, which is where the pattern came from
- [x] 6.3 Update `CLAUDE.md`: MVP item 4 partly done — definition and approval yes, execution and adapters no
- [x] 6.4 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
