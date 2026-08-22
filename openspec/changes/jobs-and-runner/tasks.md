## 1. The runner role

- [x] 1.1 Add `Role::Runner`, ordered **below** `Agent` so every existing threshold gate refuses it without change
- [x] 1.2 Add a gate that matches the runner role **exactly** — admin must not inherit it, because being the most privileged identity does not make you the machine a job was addressed to
- [x] 1.3 Confirm a runner is refused at every existing endpoint, and that no other role can reach the runner's

## 2. The job model

- [x] 2.1 Add a `Job` struct: id, grant name, parameters, runner, status, submitted_by, timestamps, result
- [x] 2.2 Status as an enum: pending, claimed, succeeded, failed — with the transitions the only way to move between them
- [x] 2.3 Create the `jobs` table, indexed on (runner, status) for the claim query
- [x] 2.4 `JobRepo`: submit, claim, report, get, and a sweep for abandoned jobs

## 3. Claiming

- [x] 3.1 Claim as a single atomic `UPDATE … WHERE id = (SELECT … LIMIT 1)`, so the write decides the winner and two runners cannot both get one job
- [x] 3.2 The claim response carries the grant's implementation and the plaintext of **only** its declared secrets
- [x] 3.3 Confirm there is no endpoint, for any role, that returns a secret by name
- [x] 3.4 Long-poll: retry every 200ms for up to 30s, then return empty
- [x] 3.5 Report accepted only from the runner that claimed the job

## 4. Execution in the runner

- [x] 4.1 `sealbox runner --name <name>`: claim, execute, report, repeat
- [x] 4.2 Materialise secrets three ways. Expressed with two grant fields rather than three: `secrets` become environment variables **and** are rendered together into an env-file at `SEALBOX_ENVFILE`, since a consumer needing `--from-env-file` should not declare the same secrets twice. `files` is for credentials that must be a file.
- [x] 4.3 Create the files in a temp directory removed by a guard, so they do not survive a panic or an early return — not a cleanup call at the bottom of the happy path
- [x] 4.4 Execute with argv, never a shell; substitute parameters into elements
- [x] 4.5 Capture exit status, stdout, and stderr; report them
- [x] 4.6 Confirm a parameter containing shell metacharacters arrives as one literal argument

## 5. Submitting and waiting

- [x] 5.1 `POST /v1/jobs` — agent and above; refuse a job naming a grant that does not exist
- [x] 5.2 Reject a submission carrying anything describing what to execute, rather than ignoring the field
- [x] 5.3 `GET /v1/jobs/{id}` for the waiting caller
- [x] 5.4 `sealbox run <grant> [key=value ...]`: submit, poll, print exit status and output
- [x] 5.5 Confirm the caller never receives a secret value

## 6. Chains and timeouts

- [x] 6.1 On success, queue the next grant in the chain — driven by the server, since a compromised runner must not be able to keep itself going
- [x] 6.2 Stop at the first failure and record which step it was
- [x] 6.3 Sweep jobs claimed but unreported past the timeout, marking them failed with a reason
- [x] 6.4 Confirm nothing is ever retried automatically

## 7. Audit

- [x] 7.1 Record submission, claim, and result as job events — "who ran what, on which runner, and what happened" should not have to be reconstructed from three URL paths
- [x] 7.2 Confirm no job event carries a secret value, including in captured output on a failure path

## 8. Tests

- [x] 8.1 End to end: approve a `script` grant, run it, get its output back
- [x] 8.2 A runner is refused at every non-runner endpoint; an admin is refused at claim
- [x] 8.3 A runner receives only the secrets its grant declares
- [x] 8.4 Two concurrent claims for one pending job: exactly one wins
- [x] 8.5 A runner may not report a job it did not claim
- [x] 8.6 A parameter with shell metacharacters is one literal argument, and nothing executes
- [x] 8.7 A file-shaped secret arrives as a path whose contents are the value, and the file is gone afterwards
- [x] 8.8 An env-file carries several secrets in `KEY=value` form
- [x] 8.9 A chain runs in order; a failing step stops it and is named
- [x] 8.10 An abandoned job is marked failed and not retried
- [x] 8.11 The caller receives an exit status and output, never a value

## 9. Documentation

- [ ] 9.1 Update `docs/cli-reference.md` for `run` and `runner`
- [ ] 9.2 Update `docs/configuration.md` for the runner's settings
- [ ] 9.3 Update `docs/getting-started.md`: the runner step is now real
- [ ] 9.4 Update `CLAUDE.md`: MVP items 5 and 6 done
- [ ] 9.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
