# The server, hosted

## Why

Everything is built and none of it runs anywhere. The server refuses to start without a master key
file, and a hosted platform has no way to put a file on a volume before the thing that mounts it
runs — the volume exists only when a machine is attached, and the machine will not stay up.

The refusal is right in every other case. Silently generating a key at a path that was mistyped
would leave every stored secret **cold**, encrypted under a key nobody has, and the failure would
surface later, on a read, to someone who did not cause it.

But that reasoning has a condition hiding in it: it is about *existing* secrets. A brand-new server
generating a key at a mistyped path makes nothing cold, because there is nothing.

## What changes

- The server generates its master key on first boot, and **only** when the database holds no master
  key and no secret, and exactly one path is configured. Every other case still refuses, loudly.
- Litestream replicates the database to object storage, supervising the server process so there is
  one process to run and no init system in the image.
- A `fly.toml` and a deployment section that is a sequence of commands, not a description of them.

## The debt this leaves, stated plainly

**The master key is on the volume and nowhere else.** Litestream replicates the database, not the
key, so losing the volume loses every secret permanently — the backup would be encrypted under a
key that went with it.

Until the recovery ceremony of [ADR 0010](../../../docs/adr/0010-recovery-via-keypair-not-a-copied-key.md)
exists, the operator backs the key up by hand, once, immediately after the first deploy. That is a
manual step in a security-critical position, which is exactly the kind of step people skip. It is
acceptable only as a stated interim, and the ceremony is the next change.

## Non-goals

- **The recovery keypair ceremony.** It is its own change, and building it half-way here would be
  worse than the honest manual step.
- **Multi-region, multi-machine, or read replicas.** One machine, one volume, one SQLite file. A
  second machine writing to a second volume is two servers disagreeing about who holds the master
  key.
