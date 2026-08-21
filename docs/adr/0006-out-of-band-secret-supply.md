# Secrets are pushed out of band; sealbox ships no cluster controller

Sealbox supplies runtime Secrets to Kubernetes by pushing them with a minimally-privileged
writer ServiceAccount, invoked as an ordinary grant. Flux continues to own everything else. We
are not making Secrets flow through Git, and we are not writing an operator, a CRD, or an
in-cluster controller.

True GitOps for secrets — encrypted material committed to Git, decrypted inside the cluster —
*requires* an in-cluster controller. That is why External Secrets Operator and Sealed Secrets
exist, and it is where their cost comes from. The author already paid that cost on this
infrastructure and reversed it: everxyz/Utopia#695 retired ESO and KMS, deleting 771 lines,
including a 379-line key-name verifier and a 162-line manifest checker. Those 541 lines were the
tax on making secrets declarative.

Out-of-band push with Flux owning the rest is already running and already has its RBAC boundary
validated. Sealbox replaces the *source of the values*, not the delivery mechanism.

## Considered Options

The status quo being replaced is GitHub Environment Secrets, which has six problems: values are
write-only (no source of truth), rotation requires a browser and a protected workflow dispatch,
agents cannot participate at all, GitHub holds every production credential, the writer kubeconfig
is itself a GitHub secret, and two writers to the same Secret force a Lease mutex.

## Consequences

Secret synchronisation moves off the release workflow and onto explicit invocation — executed by
an in-cluster runner (ADR 0008), triggered by a human or an agent. This decouples it from releases
entirely, which means **the Lease mutex guarding concurrent writes can be deleted**: the two paths
no longer touch the same object. CI never needs to reach sealbox either.

The push itself uses the runner's own ServiceAccount rather than a stored kubeconfig, scoped as
`runtime-secret-writer` is today — so the credential that used to perform this sync stops existing
rather than moving.

Purity is given up knowingly: cluster Secret state is no longer reconstructible from Git alone.
It already was not, and buying that property back costs an operator.
