# Design

## What is registered, and what is checked

An **issuer** is registered once: a name, the issuer URL as it appears in `iss`, and its public
keys as a JWKS. A **runner identity** may then be bound to one issuer and one exact subject.

```
sealbox-cli admin issuer add prod-cluster \
  --url https://kubernetes.default.svc.cluster.local \
  --jwks-file ./jwks.json      # kubectl get --raw /openid/v1/jwks

sealbox-cli admin identity create prod-runner --role runner \
  --issuer prod-cluster \
  --subject system:serviceaccount:sealbox:runner
```

Verification, on every claim:

1. the token parses as a JWT and names a registered issuer;
2. its signature checks against one of that issuer's keys;
3. `exp` is in the future and `nbf`/`iat` are not in the future, allowing for a small skew;
4. `aud` matches what the identity expects;
5. `sub` matches the bound subject **exactly** — no prefixes, no wildcards;
6. the identity is not revoked.

A failure at any step is `401` and is audited, with the step that failed as the detail. Which step
failed is safe to record: an attacker holding the token already knows.

## Exact subjects only

A prefix or a pattern would mean that creating a ServiceAccount is enough to become a runner, and
in most clusters far more people can create a ServiceAccount than can be trusted with plaintext.
One identity, one subject, and a second runner is a second identity.

## Why `aud` is required

A ServiceAccount token minted for the API server should not authenticate to sealbox. Kubernetes
projected tokens take an explicit audience:

```yaml
volumes:
  - name: sealbox-identity
    projected:
      sources:
        - serviceAccountToken:
            path: token
            audience: sealbox        # not the default
            expirationSeconds: 3600
```

Requiring it means a token stolen from somewhere else in the cluster does not work here.

## Keys are registered, not fetched

A hosted server cannot reach a private cluster's OIDC endpoint — that is the same topology
constraint that made the runner poll outbound in the first place ([ADR 0008](../../../docs/adr/0008-runner-is-the-only-executor.md)).
So the keys are supplied at registration and stored.

The cost is that a cluster rotating its signing keys needs the JWKS re-registered. That is real,
and it is the reason `issuer update` exists and why more than one key may be held at once: register
the new key beside the old one, and remove the old one when nothing presents it any more — the same
shape as master keys.

## Bearer tokens do not go away

Agents and operators keep theirs. They run on laptops and in agent harnesses that have no workload
identity to present, and inventing one for them would be a login system. What changes is the
credential that receives plaintext.

## Compatibility

A runner identity may be bound to an issuer **or** hold a token, not both. Existing runner tokens
keep working until the identity is rebound, so a cluster moves when someone moves it — but a
migration path is the only compatibility concession, and it costs nothing to keep since token
authentication has to exist for agents anyway.
