# Design

## The shape of the skill

A skill is read by an agent that has already decided to do something — provision a database, wire
a service, rotate a credential. It is not a tutorial, and it is not the CLI reference. What it
must supply is the part an agent cannot infer:

- that a secret's *value* is not obtainable, so the plan must not depend on seeing one;
- that a new credential should be **generated inside sealbox**, not invented and stored;
- that a capability needs a human signature, and the honest move is to submit and say so.

Everything else — flags, output shapes — belongs in `docs/cli-reference.md`, which the skill
links rather than duplicates. Duplicated reference material rots; this is the same reason the
examples are the template library rather than a `--from-template` flag.

## Why the refusals need explaining

An agent that hits a refusal without knowing why will route around it. Three refusals are common
enough to name in the skill, each with the reason:

| Refusal | Why it is not a bug |
|---|---|
| no command prints a secret | the point of the system |
| `secrets = { DB = "app/{env}/url" }` is refused | the parameter comes from the caller, so it would choose which credential the grant reaches |
| a submitted grant does not exist yet | a human signs for it; that is the only gate |

## Where the file lives

`skills/sealbox/SKILL.md` in this repo, so it is versioned with the CLI it describes. Installing
it is a copy into the agent's skills directory — no packaging, no registry, and no build step to
keep working.
