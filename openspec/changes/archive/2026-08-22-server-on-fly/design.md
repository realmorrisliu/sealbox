# Design

## The guard

Generation happens when all three hold:

1. exactly one path is configured — a rotation list on a fresh server is a mistake, not an intent;
2. that file does not exist;
3. the database holds no master key and no secret.

Then, and only then, the server writes a new keypair at `0600` and logs the *fingerprint* — never
the key. Anything else keeps the existing behaviour: refuse to start, and say which path was
unreadable.

Walking the cases shows why the guard is the whole design:

| Situation | Behaviour | Why it is right |
|---|---|---|
| Fresh deploy, empty volume | generates | Nothing exists to be made cold |
| Ordinary restart | loads | The file is there |
| Volume lost, database restored from Litestream | **refuses** | Generating here would be catastrophic — this is precisely when the recovery key is needed |
| Path mistyped on an existing deployment | **refuses** | Secrets exist under a key at the old path |
| Path mistyped on a fresh deployment | generates | The typo becomes the path; nothing is lost |
| Rotation list configured on a fresh server | **refuses** | Ambiguous intent, and cheap to correct |

The third row is the one that matters, and it is why the condition is *no secrets and no master
keys* rather than *no file*.

## Why not the alternatives

**A `gen-master-key` subcommand.** Still needs somewhere to run before the volume exists. It moves
the problem rather than solving it, and adds a command whose only job is to produce key material.

**The PEM in a Fly secret, injected as an environment variable.** Genuinely tempting: the key would
then live in Fly's secret store rather than beside the database, so a volume snapshot would not
contain both. It was rejected because the key would have to be *generated on the operator's
laptop* — the machine with agents on it, which is the threat the whole product exists to address.
Trading an exposure to agents for separation from a snapshot is the wrong direction, and the
snapshot case is not a new class of risk anyway: whoever can snapshot the volume can read process
memory and call the API, which is the compromised-server case ADR 0001 already accepts.

**Booting once with `fly ssh` to place the file.** Depends on a machine staying reachable while its
process crash-loops. That is platform behaviour to be observed, not a ceremony to document.

## Litestream as the supervisor

`litestream replicate -exec sealbox-server` runs replication and the server as one process tree,
with no init system in the image and no way for the server to be running while replication is not.
If Litestream exits, the server goes with it, which is the correct direction: a server writing
secrets that are not being replicated is the state that quietly costs everything later.

## One machine, deliberately

SQLite on a volume means a single writer. `fly.toml` pins one machine and auto-stop stays off: a
suspended server is a runner that cannot claim, and a rotation that has already created a database
role but cannot record it.
