## Context

See proposal.md — Why.

The constraint that determines everything here: **`rsa` has no stable release past 0.9.** The
latest published version is `0.10.0-rc.18`, a release candidate. rsa 0.9.10 depends on
`digest 0.10.7` and `rand_core 0.6.4`, which correspond to sha2 0.10 and rand 0.8.

RustCrypto is mid-migration between two generations of its trait crates. `aes-gcm` 0.11 has moved
to the new generation (`aead 0.6`, `cipher 0.5`); `rsa` has not. A project using both cannot follow
the newer generation until rsa does.

Verified before writing this: with the four versions realigned, the workspace compiles, all 80
tests pass, and clippy is clean **with no changes to any cryptographic code**. The code was always
written against these APIs — only the version numbers in `Cargo.toml` had moved.

## Goals / Non-Goals

**Goals:**

- A workspace that compiles, tests, and passes clippy, so subsequent changes can be verified.
- Dependency versions that are internally consistent and stay that way.
- A recorded contract for the stored encryption format, so the next upgrade has something to
  violate visibly rather than silently.

**Non-Goals:**

- Migrating to the newer RustCrypto generation. That is gated on rsa 0.10 being stable.
- Refactoring the CLI's output layer. It is due to be rewritten for the new command surface.
- Changing any cryptographic construction, algorithm, or stored format.

## Decisions

### Follow rsa's generation; do not mix

Align rand, sha2, and aes-gcm to what rsa 0.9 requires rather than pinning each crate
independently. `rsa` is the crate with the least freedom — it is doing the RSA-OAEP work that the
master key depends on — so it dictates the generation of the shared trait crates.

*Alternative rejected:* keep aes-gcm on 0.11 while rsa stays on 0.9. Two generations of `aead` and
`rand_core` would coexist, and the code would have to be explicit about which RNG type feeds which
primitive. In cryptographic code, that is exactly the kind of ambiguity that produces a subtle,
silent mistake.

*Alternative rejected:* move to `rsa 0.10.0-rc`. A release candidate is not an acceptable
foundation for the component that protects every credential in the system.

### Downgrade comfy-table rather than migrate

comfy-table 8.0 removed `load_preset`, used in three places in `sealbox-cli/src/output.rs`. That
module is being rewritten for the new command surface, so adapting it to an 8.0 API is work with a
known short life. Table rendering has no bearing on the product.

### Raise the MSRV to 1.93 and take clippy's advice

Both member crates omitted `rust-version`, so they did not inherit the workspace's declared
1.85.0 and clippy assumed no MSRV — which is why it proposed let-chains, stable only since 1.88.

Two ways to make clippy quiet: inherit the old MSRV so it stops suggesting newer constructs, or
raise the MSRV and adopt them. **Raise it.** Sealbox is a self-hosted tool built from source by its
operator, with CI on stable; nothing needs a 1.85 compiler. edition 2024 is already declared, and
let-chains are the natural way to write this code in that edition.

Both crates now inherit the workspace value, so this cannot drift apart again.

*Note:* the two rewritten sites are also better independently. One replaces
`!x.as_array().unwrap_or(&vec![]).is_empty()` with `x.as_array().is_some_and(|a| !a.is_empty())`,
which is equivalent and allocates nothing.

### Refresh the lockfile, which nobody had been doing

`cargo audit` reported 14 advisories. Comparing against `HEAD`'s lockfile showed the downgrades
introduced **none** of them — the set is identical before and after. They came from the TLS stack
(`reqwest` → `rustls`, `quinn`, `aws-lc-sys`) and other transitive dependencies, several rated
high.

A plain `cargo update` — no manifest changes, only the lockfile — cleared 13 of the 14, leaving one
"unmaintained" notice. The workspace still compiles and every test passes.

The gap is structural: **dependabot proposes updates to direct dependencies, so transitive security
updates were never being applied at all.** The lockfile had simply gone stale while the direct
dependencies were kept current — the opposite of the intended effect.

### Constrain dependabot rather than react to it

The cryptography crates' versions are governed by rsa, not by their own release cadence, so
proposing them individually produces pull requests that cannot be correct. They are grouped, and
major-version updates to them are not proposed automatically.

The deeper failure — four pull requests merged while CI was red — is a repository settings matter
(required status checks) that this change cannot enforce from a file. It is recorded as an operator
task instead of quietly omitted.

## Risks / Trade-offs

- **Downgrades reintroduce advisories fixed in newer versions** → `cargo audit` runs in CI and
  already tolerates one documented RSA advisory with no available fix. Any new finding surfaces
  there rather than going unnoticed.
- **Dependency versions now lag deliberately** → Recorded here and enforced in dependabot config,
  so a future reader finds a reason rather than assuming neglect.
- **Raising the MSRV excludes older toolchains** → Nothing consumes sealbox as a library, and the
  only build environments are CI on stable and the author's machine.
- **`secret-encryption` is written after the fact** → It documents current behavior rather than
  changing it, which is what makes it safe to add here. It costs nothing now and gives the next
  upgrade something explicit to test against.

## Migration Plan

No data migration; the stored format is untouched. Deployment is a rebuild.

Verification is the whole of CI, run locally before commit: `cargo fmt --all -- --check`,
`cargo build --release --workspace`, `cargo test --workspace`, `cargo test --doc --workspace
--all-features`, and `cargo clippy --all-targets --all-features --workspace -- -D warnings`.

**Rollback:** revert the commit. No persistent state is involved.

## Open Questions

None.
