## 1. Pure deletions (no behavior depends on these)

Ordered first because each is independently verifiable and none blocks the others.

- [x] 1.1 Delete the `sealbox-web` directory, remove it from the workspace members in `Cargo.toml`, and drop the pnpm workspace files
- [x] 1.2 Remove any `sealbox-web` steps, caches, or path filters from `.github/workflows/` — none existed; `sealbox-web` was never a workspace member nor referenced by CI
- [x] 1.3 Delete the CORS layer and the `SEALBOX_ALLOW_CORS` read from `sealbox-server/src/api/mod.rs`; remove `tower-http`'s `cors` feature if nothing else uses it
- [x] 1.4 Remove the `Version` enum entirely and hardcode `/v1/...` in the routes — with one variant left, the dynamic segment, the extractor, and every handler's match on it were pure noise. Also removes `MasterKeyPathParams`, `ListSecretsPathParams`, `SecretPathParams::version`, the `InvalidApiVersion` error variant, and three tests that asserted a handler-level rejection that no longer happens there
- [x] 1.5 Verify: `cargo build --release --workspace`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`

## 2. Rename rotate → rekey

Mechanical, and done before the repo refactor so the refactor does not have to move renamed code twice.

- [x] 2.1 Rename `Secret::rotate_master_key` → `Secret::rekey` in `sealbox-server/src/repo/mod.rs`, with its tests
- [x] 2.2 Rename the handler and route target in `sealbox-server/src/api/handler/master_key.rs` to rekey
- [x] 2.3 Rename the corresponding `sealbox-cli` command and its help text
- [x] 2.4 Grep for remaining uses of "rotate" that mean re-encryption and fix them, including log and error strings
- [x] 2.5 Verify: build, test, clippy clean

## 3. Repo traits own their connection

- [x] 3.1 Give `SqliteSecretRepo`, `SqliteMasterKeyRepo`, and `SqliteHealthRepo` an `Arc<Mutex<Connection>>` field and a constructor
- [x] 3.2 Remove the `&Connection` / `&mut Connection` parameter from every method on `SecretRepo`, `MasterKeyRepo`, and `HealthRepo`
- [x] 3.3 Move locking and transaction handling inside the implementors; multi-statement work takes the lock once and opens one transaction. The rekey transaction, previously assembled in the handler, becomes `SecretRepo::rekey_secrets` — which made `fetch_secrets_by_master_key` and `update_secret_master_key` dead, so both were removed along with their tests, replaced by two tests of `rekey_secrets` itself
- [x] 3.4 Remove every `state.conn_pool.lock()` from `sealbox-server/src/api/handler/*` and from the readiness probe
- [x] 3.5 Remove `conn_pool` from `AppState` if nothing outside the repos still needs it
- [x] 3.6 Verify: build, test, clippy clean — no behavior change expected, so failing tests here mean a real mistake

## 4. Schema migration

- [ ] 4.1 Add `server_held INTEGER NOT NULL DEFAULT 0` to `master_keys`
- [ ] 4.2 Rebuild `secrets` without `namespace`, with `PRIMARY KEY (key, version)`, in one transaction: create new table, copy rows, compare row counts, drop old, rename
- [ ] 4.3 Remove the `namespace` field from `Secret` and every query, insert, and test referencing it
- [ ] 4.4 Confirm the migration is idempotent on an already-migrated database and a fresh one
- [ ] 4.5 Verify: run against a copy of a real database file; row count and every secret's decryptability unchanged

## 5. Server-held master key

- [ ] 5.1 Add `SEALBOX_MASTER_KEY_PATH` to `SealboxConfig`; fail startup with a clear message if it is missing or unreadable
- [ ] 5.2 Load the private master key at startup via the existing `PrivateMasterKey`; register the corresponding public key with `server_held = 1` if not already present
- [ ] 5.3 Make "the current master key" resolve to the server-held one for new secrets
- [ ] 5.4 Reject operations that would require decrypting a secret whose master key is cold, with an error naming the cause
- [ ] 5.5 Verify: a new secret encrypts under the server-held key; a secret under a cold key reports cold rather than failing obscurely

## 6. Remove the private-key rekey endpoint

The security fix, last, because it depends on 5.

- [ ] 6.1 Delete `old_private_key_pem` from `RotateMasterKeyPayload`; the payload now names only source and destination master keys
- [ ] 6.2 Rewrite the rekey handler to use the server-held private key; reject any request whose source key is cold
- [ ] 6.3 Confirm no path logs, echoes, or stores submitted key material, including error paths
- [ ] 6.4 Update the `sealbox-cli` rekey command to stop sending a private key
- [ ] 6.5 Confirm rekey is atomic: a failure mid-operation leaves every secret on its original master key
- [ ] 6.6 Verify: build, test, clippy clean

## 7. Tests and documentation

- [ ] 7.1 Add a test that a rekey request carrying private key material is rejected and nothing is written
- [ ] 7.2 Add a test that rekey from a cold source key is refused
- [ ] 7.3 Add a test that no response carries `Access-Control-Allow-*` headers, in a debug build
- [ ] 7.4 Add a test that a non-`v1` version is rejected
- [ ] 7.5 Update `docs/configuration.md` for `SEALBOX_MASTER_KEY_PATH` and the removal of `SEALBOX_ALLOW_CORS`
- [ ] 7.6 Update `docs/cli-reference.md` for the renamed rekey command
- [ ] 7.7 Remove the stale web UI, CORS, and rotate references from `CLAUDE.md`'s repository layout and cleanup list
- [ ] 7.8 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`

## 8. Migration runbook

- [ ] 8.1 Write the operator steps into `docs/`: back up the database file, generate a server master key, set `SEALBOX_MASTER_KEY_PATH`, start, verify
- [ ] 8.2 State plainly that pre-existing secrets stay on cold keys and are re-imported with the CLI holding their private key, and why no migration path is offered
