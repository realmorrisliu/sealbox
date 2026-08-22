## 1. The runner role

- [ ] 1.1 Add `Role::Runner`, ordered **below** `Agent` so every existing threshold gate refuses it without change
- [ ] 1.2 Add a gate that matches the runner role **exactly** — admin must not inherit it, because being the most privileged identity does not make you the machine a job was addressed to
- [ ] 1.3 Confirm a runner is refused at every existing endpoint, and that no other role can reach the runner's

## 2. The job model

- [ ] 2.1 Add a `Job` struct: id, grant name, parameters, runner, status, submitted_by, timestamps, result
- [ ] 2.2 Status as an enum: pending, claimed, succeeded, failed — with the transitions the only way to move between them
- [ ] 2.3 Create the `jobs` table, indexed on (runner, status) for the claim query
- [ ] 2.4 `JobRepo`: submit, claim, report, get, and a sweep for abandoned jobs

## 3. Claiming

- [ ] 3.1 Claim as a single atomic `UPDATE … WHERE id = (SELECT … LIMIT 1)`, so the write decides the winner and two runners cannot both get one job
- [ ] 3.2 The claim response carries the grant's implementation and the plaintext of **only** its declared secrets
- [ ] 3.3 Confirm there is no endpoint, for any role, that returns a secret by name
- [ ] 3.4 Long-poll: retry every 200ms for up to 30s, then return empty
- [ ] 3.5 Report accepted only from the runner that claimed the job

## 4. Execution in the runner

- [ ] 4.1 `sealbox runner --name <name>`: claim, execute, report, repeat
- [ ] 4.2 Materialise secrets three ways: environment variable; a `0600` file whose path is substituted into argv; an env-file rendering several as `KEY=value`
- [ ] 4.3 Create the files in a temp directory removed by a guard, so they do not survive a panic or an early return — not a cleanup call at the bottom of the happy path
- [ ] 4.4 Execute with argv, never a shell; substitute parameters into elements
- [ ] 4.5 Capture exit status, stdout, and stderr; report them
- [ ] 4.6 Confirm a parameter containing shell metacharacters arrives as one literal argument

## 5. Submitting and waiting

- [ ] 5.1 `POST /v1/jobs` — agent and above; refuse a job naming a grant that does not exist
- [ ] 5.2 Reject a submission carrying anything describing what to execute, rather than ignoring the field
- [ ] 5.3 `GET /v1/jobs/{id}` for the waiting caller
- [ ] 5.4 `sealbox run <grant> [key=value ...]`: submit, poll, print exit status and output
- [ ] 5.5 Confirm the caller never receives a secret value

## 6. Chains and timeouts

- [ ] 6.1 On success, queue the next grant in the chain — driven by the server, since a compromised runner must not be able to keep itself going
- [ ] 6.2 Stop at the first failure and record which step it was
- [ ] 6.3 Sweep jobs claimed but unreported past the timeout, marking them failed with a reason
- [ ] 6.4 Confirm nothing is ever retried automatically

## 7. Audit

- [ ] 7.1 Record submission, claim, and result as job events — "who ran what, on which runner, and what happened" should not have to be reconstructed from three URL paths
- [ ] 7.2 Confirm no job event carries a secret value, including in captured output on a failure path

## 8. Tests

- [ ] 8.1 End to end: approve a `script` grant, run it, get its output back
- [ ] 8.2 A runner is refused at every non-runner endpoint; an admin is refused at claim
- [ ] 8.3 A runner receives only the secrets its grant declares
- [ ] 8.4 Two concurrent claims for one pending job: exactly one wins
- [ ] 8.5 A runner may not report a job it did not claim
- [ ] 8.6 A parameter with shell metacharacters is one literal argument, and nothing executes
- [ ] 8.7 A file-shaped secret arrives as a path whose contents are the value, and the file is gone afterwards
- [ ] 8.8 An env-file carries several secrets in `KEY=value` form
- [ ] 8.9 A chain runs in order; a failing step stops it and is named
- [ ] 8.10 An abandoned job is marked failed and not retried
- [ ] 8.11 The caller receives an exit status and output, never a value

## 9. Documentation

- [ ] 9.1 Update `docs/cli-reference.md` for `run` and `runner`
- [ ] 9.2 Update `docs/configuration.md` for the runner's settings
- [ ] 9.3 Update `docs/getting-started.md`: the runner step is now real
- [ ] 9.4 Update `CLAUDE.md`: MVP items 5 and 6 done
- [ ] 9.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
