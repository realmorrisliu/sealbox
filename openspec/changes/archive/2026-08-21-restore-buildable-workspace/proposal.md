## Why

**The workspace does not compile.** `main` has been broken since at least 2026-08-10, and four
dependabot pull requests were merged into it during that window with their CI runs already red.

```
error[E0432]: unresolved import `aes_gcm::aead::OsRng`
error[E0425]: cannot find function `thread_rng` in crate `rand`   (×3)
error[E0599]: no function or associated item named `generate_nonce` found for AesGcm
error[E0277]: the trait bound `Sha256: Digest` is not satisfied    (×2)
error[E0599]: no method named `load_preset` found for comfy_table::Table  (×3)
```

The cause is a version split in the RustCrypto ecosystem. `rsa` has no stable release past 0.9
(only `0.10.0-rc.18`), and rsa 0.9 pins `digest 0.10` and `rand_core 0.6` — meaning sha2 0.10 and
rand 0.8. Dependabot raised the direct dependencies to rand 0.10, sha2 0.11, and aes-gcm 0.11,
which are built on the next generation of those traits. The versions in `Cargo.toml` and the
versions in `Cargo.lock` stopped agreeing, and the code was never migrated — only the version
numbers changed.

This blocks everything. Every task group in `retire-legacy-surface` ends with "build, test, clippy
clean"; with a broken build, no change to this repository can be verified at all.

Fixing the compile without fixing the process that broke it would buy about a week.

## What Changes

- Realign the cryptography dependencies with the generation `rsa` 0.9 requires: `rand` 0.10 → 0.8,
  `sha2` 0.11 → 0.10, `aes-gcm` 0.11 → 0.10. **No code changes** — the code was always written
  against these APIs.
- Downgrade `comfy-table` 8.0 → 7, which restores `load_preset`. The CLI's output layer is due to
  be rewritten for the new command surface; migrating it to an 8.0 API first would be wasted work.
- **BREAKING (for contributors)** — raise the MSRV from 1.85.0 to 1.93.0, and have both member
  crates inherit it via `rust-version.workspace = true`. Neither did, so clippy assumed no MSRV and
  proposed let-chains, a feature that did not stabilise until 1.88.
- Adopt those suggestions rather than suppress them: rewrite two nested `if let` blocks as
  let-chains, and annotate one elided lifetime that clippy 1.93 newly flags.
- Prevent recurrence: group the cryptography crates in dependabot and stop major-version bumps to
  them from being proposed automatically, since their versions are governed by `rsa`, not by their
  own release cadence.

## Capabilities

### New Capabilities

- `secret-encryption`: the envelope encryption contract — what is stored, and the guarantee that
  data written by an earlier build remains readable by a later one. Recorded now because this
  incident showed that nothing states it, so a dependency bump can silently threaten it.

### Modified Capabilities

None.

## Impact

**Constrained by** nothing in `docs/adr/`; this change records no new direction.

**Blocks** `retire-legacy-surface`, and every change after it.

**Code**
- `Cargo.toml` — four dependency versions, MSRV
- `sealbox-cli/Cargo.toml`, `sealbox-server/Cargo.toml` — inherit MSRV
- `sealbox-server/src/repo/mod.rs` — one lifetime annotation
- `sealbox-cli/src/config.rs`, `sealbox-cli/src/commands/key_commands.rs` — two let-chain rewrites
- `.github/dependabot.yml` — grouping and major-version policy
- `README.md` — the Rust version badge

**Not affected** — every stored secret. The cryptographic construction, the algorithms, and the
stored format are unchanged; this restores the versions the code was already written against
rather than migrating to different ones.

**Process** — merging a pull request whose CI is failing is what allowed this. The repository
settings that permit it are outside this change's reach, but the change records the requirement.
