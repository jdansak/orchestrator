# Security Policy

## Reporting a vulnerability

**Do not open a public issue for a security vulnerability.**

Report it privately through GitHub's security advisory form:

> **Security** → **Report a vulnerability** → https://github.com/Jdansak/orchestrator/security/advisories/new

Please include:

- A description of the vulnerability and what an attacker could do with it
- The affected version, commit, or configuration
- Steps to reproduce, or a proof of concept
- Any suggested mitigation

We will acknowledge the report within **3 business days**, send an initial assessment within
**10 business days**, and keep you updated as the fix progresses.

If GitHub's advisory form is not suitable, email the maintainers directly at
`security@orchestrator.dev` with the same details.

## Disclosure timeline

| Stage | Target |
|-------|--------|
| Acknowledgement | 3 business days |
| Initial assessment | 10 business days |
| Fix released | 30 days from the report, sooner if a fix is ready |
| Public disclosure | 90 days from the report, or immediately when the fix ships — whichever comes first |

We will credit you in the advisory unless you ask us not to.

## Supported versions

The project is pre-alpha. Until the first stable release, fixes land on `main` and there are no
long-term support branches.

| Version | Supported |
|---------|-----------|
| `main` | ✅ |
| `< 0.1.0` releases | ❌ |

## Security-relevant areas

These are the parts of the system most worth attacking. If you find something wrong in one of
them, please report it even if the impact seems small.

| Area | Why it matters |
|------|----------------|
| **Configuration parsing** | A tenant config is the primary input to the data plane. A parser bug can reach the proxy, the DNS server, and the VIP manager |
| **The REST APIs** | The manager API controls the tenant lifecycle; the tenant API controls traffic and rollouts. Both are authenticated |
| **Authentication** | Argon2id password hashing, opaque bearer tokens, constant-time comparison |
| **TLS termination** | rustls, TLS 1.2 minimum, optional client-certificate verification |
| **DNS serving** | The server is authoritative for tenant zones. Zone transfers and `ANY` queries that leak the zone are refused |
| **The proxy** | Client-supplied `X-Forwarded-*` headers must never be trusted |
| **The worker sync channel** | A compromised peer could influence routing or force a failover |
| **VIP manipulation** | Runs with `CAP_NET_ADMIN`. A bug here could take over addresses belonging to other tenants |

## Hardening assumptions

The design makes these assumptions. If you find a way to violate one, that is a bug worth
reporting.

- **No secret material lives in a config file.** Certificates are referenced by path; passwords
  live in a separate `0600` credential file.
- **Tenants are isolated.** A tenant's process cannot read another tenant's configuration, and a
  crash in one tenant cannot affect another.
- **The metrics listener is not public.** It binds to loopback by default.
- **A validation failure never changes the running configuration.** A bad config is rejected and
  the previous one keeps serving.
- **The DNS server does not answer zone transfers.**
- **Backend TLS verification is on by default** and cannot be silently disabled.
