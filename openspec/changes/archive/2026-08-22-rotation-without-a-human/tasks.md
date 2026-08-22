## 1. Rotation is not an operator's privilege

- [x] 1.1 Move `rotate` to the group that may invoke grants
- [x] 1.2 Replace the test asserting an agent cannot rotate with one asserting it can, and record why in the test itself
- [x] 1.3 Check nothing else assumed the operator boundary

## 2. `rotate_after`

- [x] 2.1 Store a rotation interval on a secret
- [x] 2.2 Carry it forward on rotation — losing it at the first rotation would be the worst moment
- [x] 2.3 Accept it on `secret set` and `secret gen`, in the same duration form `audit --since` takes
- [x] 2.4 Show it, and when the secret is next due, on `secret show`

## 3. Overdue

- [x] 3.1 `GET /v1/secrets?overdue=true`, computed at read time
- [x] 3.2 `sealbox-cli secret list --overdue`
- [x] 3.3 Say plainly in the CLI where someone would otherwise reach for `--ttl`

## 4. The principle

- [x] 4.1 ADR 0013: automate first, agent-ify what needs judgement, and keep the human for irreversible widening of authority
- [x] 4.2 Point the design document's permission table and `SKILL.md` at what is now true

## 5. Tests

- [x] 5.1 An agent rotates through an approved grant
- [x] 5.2 A rotation carries the interval forward
- [x] 5.3 Overdue lists exactly what is past its interval, and nothing without one
- [x] 5.4 Rotating settles it
- [x] 5.5 Nothing acts on the interval on its own

## 6. Verification

- [x] 6.1 `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
- [x] 6.2 Re-run the local end-to-end: an agent token rotates through `pg-provision` and the result reaches Kubernetes
