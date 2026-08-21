# Sealbox

A broker that sits between agents and real credentials. Agents can *use* a credential
without ever *seeing* it.

## Language

**Secret**:
A stored credential value. Always enveloped at rest, and never returned to any caller — the only
place its plaintext appears outside the server is inside a Runner executing a Grant.
_Avoid_: password, key (too overloaded), config value

**Grant**:
A permitted use of one or more secrets: which secrets it needs, what it does with them, and on
which runner. Everything a secret can be used for equals the set of grants that declare it.
_Avoid_: task, action, script, recipe. Not to be confused with Job — a grant is the standing
permission, a job is one execution of it.

**Adapter**:
A built-in, reusable implementation of a grant for one class of target system. Structurally
limited to what that class of system needs, unlike a script.
_Avoid_: plugin, engine, provider, driver

**Identity**:
A named caller of sealbox with a role and its own means of authenticating: humans register a
passkey, while agents and runners hold bearer tokens because they have no fingers. Revocable
individually.
_Avoid_: user, account, client, principal

**Master Key**:
The keypair a secret's data key is encrypted under. Server-held ones make a secret usable by
the broker; the rest are cold, including the Recovery Key.
_Avoid_: KEK, root key, encryption key — "master key" is the only name for this, at every layer

**Server-held**:
Describes a master key whose private half the sealbox server possesses, and therefore any
secret encrypted under it. The precondition for every broker feature.
_Avoid_: managed, hot, unsealed

**Cold**:
Describes a secret the server cannot decrypt, because its master key is not server-held. Only
a holder of the offline private key can read it. The recovery and high-value path.
_Avoid_: archived, offline, sealed

**Recovery Key**:
The master key generated on the operator's own machine at initialisation, whose private half the
server never sees. It exists so the server's own master key can be stored encrypted and restored
after total loss. A specific, mandatory instance of a Cold master key.
_Avoid_: backup key, escrow key, break-glass key

**Rotate**:
Replacing a secret's *value* with a newly generated one, and pushing that value to whatever
system must accept it, as a single all-or-nothing step.
_Avoid_: refresh, cycle, renew — and never use it for changing master keys, which is Rekey

**Rekey**:
Re-encrypting a secret's data key under a different master key. The secret's value does not
change.
_Avoid_: rotate (reserved for values), re-encrypt

**Runner**:
A process that long-polls the server for jobs and executes them. The only place a secret's
plaintext exists outside the server, and the only place grants run.
_Avoid_: worker, agent (means something else here), executor

**Job**:
One requested execution of a grant: its parameters, the runner it is addressed to, and its result.
_Avoid_: run, invocation, execution

**Lease**:
A time-boxed, single-use release of an actual plaintext value to an identity. Considered and not
implemented — the Runner model removes the need, since a caller never has to hold a value to use
it. Recorded here so the term is not reintroduced with a different meaning.
_Avoid_: checkout, borrow. Never a synonym for Grant.

**Capture**:
Storing whatever a grant printed as the secret's new value, so that a credential composed or
issued outside sealbox never passes through an agent's context. A *mode of Rotate*
(`rotate --from-output`), not a command and not a separate concept.
_Avoid_: import, ingest, scrape
