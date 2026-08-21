## 1. Realign dependency versions

- [x] 1.1 In the workspace `Cargo.toml`, set `rand = "0.8"`, `sha2 = "0.10"`, `aes-gcm = "0.10"` — the generation `rsa` 0.9 requires
- [x] 1.2 In `sealbox-cli/Cargo.toml`, set `comfy-table = "7"`
- [x] 1.3 Confirm no cryptographic code changed: `git diff --stat` shows no edits under `sealbox-server/src/crypto/`
- [x] 1.4 Confirm `cargo build --workspace` succeeds

## 2. MSRV and the lints it was hiding

- [x] 2.1 Raise `rust-version` to `1.93.0` in the workspace `Cargo.toml`
- [x] 2.2 Add `rust-version.workspace = true` to both `sealbox-server/Cargo.toml` and `sealbox-cli/Cargo.toml`, so the declared MSRV cannot drift from what clippy assumes
- [x] 2.3 Annotate the elided lifetime clippy 1.93 flags in `sealbox-server/src/repo/mod.rs` (`ToSqlOutput<'_>`)
- [x] 2.4 Rewrite the nested `if let` in `sealbox-cli/src/config.rs` as a let-chain
- [x] 2.5 Rewrite the nested `if let` in `sealbox-cli/src/commands/key_commands.rs` as a let-chain, replacing `unwrap_or(&vec![])` with `is_some_and`
- [x] 2.6 Update the Rust version badge in `README.md`

## 3. Stop it happening again

- [x] 3.1 Group `rsa`, `rand`, `sha2`, and `aes-gcm` in `.github/dependabot.yml`, so they are never proposed individually
- [x] 3.2 Ignore major-version updates for that group: their versions are governed by `rsa`, not their own release cadence
- [x] 3.3 Add a comment in `dependabot.yml` explaining the constraint, so the next person to loosen it knows what it protects
- [x] 3.4 Record the operator task: require passing status checks before merge on `main`, since four pull requests were merged with CI red — this cannot be enforced from a file in the repository

## 4. Verify the whole of CI locally

- [x] 4.1 `cargo fmt --all -- --check`
- [x] 4.2 `cargo build --release --workspace`
- [x] 4.3 `cargo test --workspace` — 80 tests expected
- [x] 4.4 `cargo test --doc --workspace --all-features`
- [x] 4.5 `cargo clippy --all-targets --all-features --workspace -- -D warnings` — zero warnings
- [x] 4.6 `cargo audit --ignore RUSTSEC-2023-0071` — confirmed against `HEAD`'s lockfile that the downgrades introduce **zero** new advisories; the set is identical
- [x] 4.7 Run `cargo update` to refresh transitive dependencies, which dependabot never touches — clears 13 of 14 pre-existing advisories, leaving one unmaintained notice
- [x] 4.8 Re-run the full verification after the lockfile refresh
