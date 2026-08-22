# A runner proves who it is instead of holding a secret

## Why

A runner's credential is the one credential that **receives plaintext**. Today it is a long-lived
bearer token, created once and pasted into a Kubernetes Secret, where it stays until someone
remembers to replace it. Anyone who can read Secrets in that namespace can claim the jobs
addressed to that runner and be handed the values.

This is the last item in the MVP, and by [ADR 0013](../../../docs/adr/0013-automation-first.md)'s
measure it is one of only two places left where a person maintains something no rule says they must.

## The recorded design does not survive a restart

[`docs/agent-native-design.md`](../../../docs/agent-native-design.md) specifies a **15-minute,
single-use join token**, exchanged at first start for a keypair the runner generates itself. The
Secret holding the join token is worthless minutes later, which is the appeal.

It does not say where that keypair lives, and there is no answer that works:

- **In memory** — a pod restart loses it, and the join token that would let it re-register expired
  fifteen minutes after it was created. The runner never comes back.
- **On a volume** — the long-lived credential is back, moved from a Secret (encrypted at rest,
  RBAC-controlled, revocable) to a PVC, which is worse in every one of those respects.
- **A reusable join token** — then it is a long-lived bearer token wearing a different name.

Pods restart. Nodes drain. This is not an edge case, and it is why the join-token design should not
be built as recorded.

## What is proposed instead

**The runner presents the identity its platform already gives it, and sealbox verifies it offline.**

A Kubernetes ServiceAccount token is a JWT the cluster signs, mounted into the pod, rotated by the
kubelet, and reissued on every restart. Sealbox does not need to reach the cluster to check one: it
needs the issuer's public keys, registered once. The same mechanism covers GitHub Actions, GCP, AWS
IRSA, and anything else that speaks OIDC — this is workload identity, not Kubernetes support, and
no provider-specific code enters sealbox ([the standing rule](../../../CLAUDE.md)).

What that removes:

- No credential in a Secret. Nothing to leak, rotate, or remember.
- Restart-safe: the token is remounted and refreshed by the platform.
- Revocation stays sealbox's: a runner identity is bound to one issuer and one subject, and
  revoking the identity ends it regardless of what the platform still hands out.

What it costs: **one registration per cluster**, by a human — the issuer and its keys, and which
subject may be which runner. That is authority being widened, which is exactly where ADR 0013 says
a person belongs.

## Alternatives

**Join token plus a persisted keypair.** Smaller, and it matches what is written down — but it
needs a volume, and the credential is on disk again. It trades a Secret for a PVC.

**Leave it.** A long-lived token is *revocable and audited*, which is more than most deployments
manage. The honest case for waiting is that there is one runner and one operator today. The case
against is that this credential receives plaintext, and it is the only one that does.

## Not in scope

- Fetching JWKS over the network. A private cluster's issuer is not reachable from a hosted
  server, so keys are registered rather than discovered. Discovery can come later for public
  issuers, and would change no behaviour here.
- Mutual TLS, SPIFFE, or an agent identity beyond what already exists.
