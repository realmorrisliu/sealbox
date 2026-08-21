# Broker model over end-to-end encryption

Sealbox's original design had clients decrypt secrets with their own RSA private keys. That is
incompatible with serving agents: an agent can only decrypt if it holds the private key, and
distributing a private key is strictly worse than distributing a secret, because it opens every
historical version. We chose to let the server hold a key and decrypt, so that agents can use
credentials through the server without ever receiving plaintext.

The client-decrypt path survives as the *cold* path for high-value, human-held credentials and
for recovery — a distinction that falls out of a single `master_keys.server_held` column rather
than a new mechanism.

## Considered Options

Keeping strict E2EE was considered and rejected. It would have limited sealbox to agents running
on the same machine as the private key, ruling out the broker, the egress proxy, and any hosted
offering. The cost of giving it up was also lower than it appeared: the pre-existing
implementation already sent plaintext to the server on write, and required clients to POST their
old private key in the clear during rotation.

## Consequences

The server can read any non-cold secret. For the hosted service this is addressed by defaulting
secrets to non-server-held, so the operator hosts ciphertext unless a user opts a specific secret
into broker features.

Because plaintext never leaves sealbox, the database file becomes the only copy of some
credentials. A cold-path recovery export is therefore mandatory, not optional; ADR 0010 specifies
it as a recovery keypair generated on the operator's machine — which is simply a master key with
`server_held = 0`, so recovery reuses this decision's mechanism rather than adding one.
