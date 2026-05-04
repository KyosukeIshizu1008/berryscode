# Security Policy

BerryCode is a desktop IDE that touches your filesystem, runs subprocesses
(LSPs, `cargo`, Git), and — when you opt in — talks to AI provider APIs
with your keys. We take that surface area seriously even though the
project is small.

## Supported versions

Only the **latest minor release line** receives security fixes. Once a
new minor version ships, the previous minor moves to "best effort" —
we'll usually backport critical fixes for ~30 days, but no guarantee.

| Version | Supported |
|---------|-----------|
| 0.8.x | ✅ active |
| 0.7.x | ⚠️ best-effort, ~30 days post-0.8.0 |
| ≤ 0.6.x | ❌ end-of-life |

If you're packaging BerryCode (Snap, Flatpak, Homebrew, AUR), please
follow the latest stable release.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security problems.**

### Preferred: GitHub Private Vulnerability Report

1. Go to
   https://github.com/KyosukeIshizu1008/berryscode/security/advisories/new
2. Fill out the form. The maintainer is notified privately and we work
   the fix in a temporary fork.

This is the canonical path. The reporter, the timeline, and the
discussion all stay private until the fix is published.

### Fallback: email

If you can't use GitHub Security Advisories for any reason, email
**ishizu@oracleberry.co.jp** with:

- A description of the issue.
- Steps to reproduce (or a proof-of-concept that reproduces it).
- BerryCode version + OS.
- Whether you want public credit when the fix ships.

PGP / GPG is available on request — email and ask for a key.

### What to expect

| Step | Target time |
|------|-------------|
| Acknowledgement that we received the report | within 48 hours |
| Initial assessment (severity, reproducibility) | within 1 week |
| Fix or mitigation in a stable release | depends on severity, see below |
| Public disclosure | after fix ships, by mutual agreement |

#### Severity targets

- **Critical** (RCE, auth bypass, data exfiltration): patch in
  ≤ 14 days, ideally ≤ 7.
- **High** (privilege escalation, denial of service of the host
  machine): patch in ≤ 30 days.
- **Moderate / Low** (info leak, crash with no security impact, etc.):
  bundled into the next regular release.

We won't sit on a fix to align with marketing — security releases ship
out-of-cadence as soon as they're ready.

### Disclosure policy

We follow **coordinated disclosure**. The default timeline is:

1. Reporter contacts us (private channel).
2. We acknowledge, work the fix, ship a release.
3. We publish a GitHub Security Advisory **after** the fix is in users'
   hands, crediting the reporter (unless they request anonymity).

If we go silent for more than 30 days after the initial acknowledgement
without a clear reason, the reporter is free to disclose publicly. We'd
much rather you ping us first, but we won't legally chase a reporter
who acts in good faith.

## Out of scope

The following are **not** security issues for this project:

- A user with shell access to the same machine being able to modify
  BerryCode's config files. (BerryCode is a desktop app; if a hostile
  process has shell access, it owns the machine, not us.)
- Theoretical key leakage by AI providers — we round-trip API keys
  directly to the provider per BYOK; what they do with them is their
  policy, not ours.
- Performance issues that aren't denial-of-service. Open a regular
  issue for those.
- Findings against dependencies that don't affect us in practice
  (we'll still want to know, but they're advisories, not vulnerabilities
  in BerryCode itself).

## Hall of fame

Once we have any reports, we'll list them here (with consent) — name,
date, advisory link.

_(Empty so far — be the first.)_

## Out-of-band channels

- General Discord: https://discord.gg/u5VYs7za (do **not** post
  security issues there; this is for general support).
- Maintainer email: ishizu@oracleberry.co.jp.
