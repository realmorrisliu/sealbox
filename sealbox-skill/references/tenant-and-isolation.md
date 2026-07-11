# Multi-tenant and backward-compatibility notes

Sealbox supports both legacy static-token workflows and tenant-isolated v2 APIs.

## Tenant model

- v2 routes derive a tenant context from tenant bearer token.
- Tenant context is implicit in the token and used to scope secret/credential/master-key data.
- Root token is for admin operations and is separated from regular tenant operations.

## Tenant operations (admin)

- `POST /v2/admin/tenants` creates tenant.
- `GET /v2/admin/tenants` enumerates tenants.
- `GET /v2/admin/tenants/:tenant_id` reads tenant metadata.
- `POST /v2/admin/tenants/:tenant_id/suspend` disables tenant access.
- `POST /v2/admin/tenants/:tenant_id/resume` re-enables tenant access.

## Tenant token lifecycle

- `POST /v2/admin/tenants/:tenant_id/tokens` creates a token.
- `GET /v2/admin/tenants/:tenant_id/tokens` lists tokens.
- `DELETE /v2/admin/tenants/:tenant_id/tokens/:token_id` revokes a token.

## v1 and v2 compatibility

- `v1` is legacy/compatibility oriented and continues to support existing workflows.
- `v2` introduces tenant-scoped separation for stronger multi-tenant isolation.
- Existing CLI and automation can remain v1 until tenant scope is required.
- `SEALBOX_API_VERSION` controls CLI routing; do not mix token types at one setting.

## Operational patterns

- Use one token class per environment (dev/test/prod) and per client role.
- Store tenant service tokens separately from administration/root token.
- Keep tenant tokens rotate-friendly: revoke and recreate when suspicious activity is suspected.
- If a tenant migration is underway, keep tenant and non-tenant tasks clearly separated in scripts and CI.
