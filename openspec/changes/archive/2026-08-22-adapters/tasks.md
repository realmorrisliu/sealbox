## 1. Configuration validation

- [x] 1.1 A typed configuration struct per adapter, with `deny_unknown_fields` — a typo in `namespace` that silently wrote to `default` would be found in production, by someone who did not make it
- [x] 1.2 Validate an adapter's configuration at grant creation, naming what is missing or out of range
- [x] 1.3 Close the privilege set for `postgres-role`: `CONNECT`, `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `USAGE`; refuse anything else at creation
- [x] 1.4 Confirm no configuration field can carry a command, a query, a resource kind, or a verb

## 2. kubernetes-secret

- [x] 2.1 Build a fixed argv: `kubectl create secret generic <name> -n <namespace> --from-env-file=… --dry-run=client -o yaml | kubectl apply -f -`, with the verb and resource kind fixed in code
- [x] 2.2 Use the env-file the runner already materialises, so the adapter needs no separate rendering
- [x] 2.3 Replace rather than merge, so removing a secret from the grant removes it from the cluster
- [x] 2.4 Fail with a clear message when `kubectl` is missing, rather than a confusing one

## 3. postgres-role

- [x] 3.1 Find the next role name by prefix and serial, so the grant stays stable across rotations and can be approved once
- [x] 3.2 `CREATE ROLE <prefix>_<n> LOGIN PASSWORD …` with the value provided as `SEALBOX_NEW`
- [x] 3.3 Grant only the configured privileges, matched against constants and never concatenated from input
- [x] 3.4 Never `ALTER` an existing role's password and never `DROP` one — mutating in place has a window where the database and the cluster disagree, and every request in it fails
- [x] 3.5 Emit a connection URL with the password percent-encoded, and nothing else on stdout, so it works with `--from-output`
- [x] 3.6 Fail clearly when `psql` is missing

## 4. Tests

- [x] 4.1 An unknown configuration field is refused at grant creation
- [x] 4.2 A privilege outside the closed set is refused, and the error names what is permitted
- [x] 4.3 A missing required setting is refused and named
- [x] 4.4 The argv `kubernetes-secret` builds is exactly what is expected, with no configuration value reaching the verb or resource kind
- [x] 4.5 The SQL `postgres-role` builds contains no interpolated privilege string
- [x] 4.6 The emitted URL percent-encodes a password containing characters that would otherwise break it
- [x] 4.7 Role naming picks the next serial rather than reusing one

## 5. Documentation

- [x] 5.1 Document both adapters in `docs/cli-reference.md`: their settings and what they are structurally unable to do
- [x] 5.2 Note the runner image's requirement for `kubectl` and `psql` in `docs/configuration.md`
- [x] 5.3 Update `examples/grants/` so the shipped examples are runnable
- [x] 5.4 Update `CLAUDE.md`: MVP item 4 complete
- [x] 5.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
