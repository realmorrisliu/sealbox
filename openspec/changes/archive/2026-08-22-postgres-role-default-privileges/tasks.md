## 1. The grant

- [x] 1.1 `postgres-role` takes a required `owner`
- [x] 1.2 Validate it as an identifier, at approval, while a person is present
- [x] 1.3 Refuse a grant without one, saying what breaks if it were allowed

## 2. The SQL

- [x] 2.1 Keep `ON ALL TABLES` for what already exists
- [x] 2.2 Add `ALTER DEFAULT PRIVILEGES FOR ROLE <owner>` for what comes later
- [x] 2.3 Sequences alongside tables, in both forms
- [x] 2.4 Still one transaction

## 3. Tests

- [x] 3.1 Both forms are emitted, and name the owner
- [x] 3.2 A grant with no table privileges emits neither table statement
- [x] 3.3 Sequence grants follow a write privilege and not a read-only one
- [x] 3.4 An owner that could carry SQL is refused

## 4. Against a real database

- [x] 4.1 Provision **before** any table exists, then create one as the owner, and confirm the role can read and write it — the case that was broken
- [x] 4.2 Provision **after** tables exist and confirm the same
- [x] 4.3 Confirm inserting into a table with a generated key works

## 5. Documentation

- [x] 5.1 Update `examples/grants/pg-provision.toml`
- [x] 5.2 Say in the CLI reference what `owner` is and why it cannot be guessed
- [x] 5.3 Record the membership requirement, and what to do when it fails

## 6. Verification

- [x] 6.1 `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
