# No web UI

Sealbox's entire interface is the server plus the CLI. The React web UI is removed and not
replaced, not even by a read-only dashboard.

Agents reach sealbox through the CLI and a skill; humans do their day-to-day work through the
same CLI. No *interface* in the product needs a browser, and a secret broker pays a real cost for
having one: CORS, session handling, a token in browser storage, and a rendering stack are all
attack surface that exists solely to serve a view `sealbox audit` already prints in a terminal.

## Amended by ADR 0009 — the approval page

ADR 0009 introduced exactly one server-rendered page, at `/approve/<id>`, and this decision has
to state why that is not a contradiction.

It is not an interface; it is part of an authentication ceremony, and it exists because a
terminal cannot provide a **trusted display**. Terminal output is written by whatever process is
running, so an agent could show one grant and submit another. A page rendered by the server
cannot be influenced that way, so what a human reads is what they sign.

Its shape is constrained accordingly: one server-rendered page plus WebAuthn JavaScript, **no
session storage, no stored token, no SPA, and no CORS** — it is same-origin, so removing the CORS
layer from the server remains correct. Nothing may be added to it that a human could *manage*
sealbox with. The moment it grows a secret list, it has become the web UI this ADR rejects.

## Considered Options

Keeping a read-only audit/status dashboard was considered — it was the previous plan — and
rejected. A dashboard is for showing things to other people, and at MVP there are no other
people. It can be reconsidered if a hosted offering ever has users who are not operators.

## Consequences

Deletes ~4500 lines of TypeScript and ~660 lines of translations across four languages, one
pnpm workspace, and its share of dependency-update churn. Also allows the CORS layer to be
removed from the server entirely.
