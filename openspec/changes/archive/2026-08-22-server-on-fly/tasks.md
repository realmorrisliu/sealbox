## 1. First-boot generation

- [x] 1.1 Generate at the configured path when exactly one is configured, no file exists, and the store holds no master key and no secret
- [x] 1.2 Write it `0600`; log a fingerprint, never the key
- [x] 1.3 Keep every other case fatal, with the message naming the path
- [x] 1.4 A secret repository count, so the guard asks the question directly rather than inferring it from a listing

## 2. The image and the platform

- [x] 2.1 Litestream in the image, supervising the server with `replicate -exec`
- [x] 2.2 `fly.toml`: one machine, a volume, no auto-stop, health checks against `/healthz/*`
- [x] 2.3 Fix the Dockerfile health check, which probes `/` rather than the liveness route

## 3. Documentation

- [x] 3.1 A deployment section that is a sequence of commands
- [x] 3.2 State the master-key backup step, and that skipping it loses everything if the volume goes
- [x] 3.3 Say what Litestream does and does not cover — the database, not the key

## 4. Tests

- [x] 4.1 A fresh store generates and starts
- [x] 4.2 A store with secrets and no key file refuses
- [x] 4.3 A configured rotation list on a fresh store refuses
- [x] 4.4 Generation is not repeated on the next start

## 5. Verification

- [x] 5.1 `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
