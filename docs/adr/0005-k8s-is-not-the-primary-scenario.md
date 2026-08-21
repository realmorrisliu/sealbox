# Operating Kubernetes is not the primary scenario; supplying its Secrets is

Two different things get called "the k8s use case", and they point in opposite directions.

**Operating a cluster** — an agent running `kubectl` — is *not* a sealbox scenario. **Supplying
the Secrets that workloads in the cluster consume** *is*, and it is the primary one. See ADR
0006.

The reasoning that rules out the first:

1. **The stated problem does not exist there.** An agent runs `kubectl`; `kubectl` reads
   `~/.kube/config` itself. The credential never enters the agent's context, so "the agent must
   not see the credential" solves nothing.
2. **The real risk is capability, not disclosure** — an agent with kubectl can `delete ns prod`,
   or read every Secret in the cluster with `get secret -o yaml` and *thereby* pull them into its
   context.
3. **But Kubernetes already solves capability, and solves it better.** A RBAC-restricted
   ServiceAccount is enforced server-side; an agent with a shell cannot get around it. Sealbox's
   named grants are a client-side constraint on a machine the agent shares. Server-side
   enforcement beats client-side constraint, so RBAC wins outright.

None of this applies to Secret *supply*. RBAC governs what an identity may do to the cluster; it
says nothing about where a database password comes from, who may read it, or how it is rotated.
There sealbox has no competitor that is not an order of magnitude heavier.

## Consequences

kubeconfigs belong in sealbox — as one credential type among others, and because a kubeconfig
left at `~/.kube/config` lets an agent bypass named grants entirely. Ad-hoc cluster operation
stays a human job with an RBAC-restricted ServiceAccount.
