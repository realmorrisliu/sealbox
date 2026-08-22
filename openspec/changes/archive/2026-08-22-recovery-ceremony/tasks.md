## 1. Storing the blob

- [x] 1.1 A recovery blob: which recovery key it is under, the encrypted master key, and the encrypted data key
- [x] 1.2 Registering a recovery public key stores it as a master key the server does not hold — the cold path, reused
- [x] 1.3 One blob per registered recovery key; registering one does not disturb another

## 2. Keeping it current

- [x] 2.1 Re-make every blob when the server's master key changes
- [x] 2.2 Cover both paths that change it: creating a master key and rekeying onto one

## 3. Endpoints

- [x] 3.1 Register a recovery key — admin
- [x] 3.2 Export a blob — admin
- [x] 3.3 Nothing returns the master key in any other form, and nothing logs it

## 4. Client

- [x] 4.1 `recovery init` — generate locally, write the private half `0600`, upload the public half
- [x] 4.2 Verify by decrypting the stored blob with the file just written, and fail loudly if it does not
- [x] 4.3 `recovery export`
- [x] 4.4 `recovery restore` — blob plus key to `master.pem`, with no server involved
- [x] 4.5 Say plainly what the file is and where it belongs

## 5. Tests

- [x] 5.1 A blob round-trips to the original master key
- [x] 5.2 A blob alone yields nothing
- [x] 5.3 The wrong recovery key fails cleanly rather than producing rubbish
- [x] 5.4 Changing the master key refreshes every blob
- [x] 5.5 Two recovery keys each recover independently
- [x] 5.6 No endpoint returns the master key unencrypted

## 6. Documentation

- [x] 6.1 Replace the manual `fly ssh cat` step in `docs/getting-started.md`
- [x] 6.2 Record where this departs from ADR 0010's re-entry, and why
- [x] 6.3 Update the status banners: this was the last unbuilt MVP item

## 7. Verification

- [x] 7.1 `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
- [x] 7.2 End to end locally: initialise recovery, delete the master key file, restore it from the blob, and read a secret written before the loss
