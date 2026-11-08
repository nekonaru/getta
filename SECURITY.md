# Security Policy

Getta takes security seriously. This document explains how to report a vulnerability and what to expect.

## Reporting a vulnerability

Do not open a public issue for a vulnerability. Use GitHub's private vulnerability reporting instead:

https://github.com/nekonaru/getta/security/advisories/new

Please include:

- What you found and where (file, function, commit).
- Steps to reproduce or a proof of concept.
- Impact and how an attacker could use it.

You will get an acknowledgment within 7 days. If the report is confirmed, a fix is prepared and released as soon as reasonably possible, and you will be credited in the advisory unless you prefer otherwise.

## Scope

In scope:

- The Getta desktop app (this repository).
- The build and release pipeline (GitHub Actions) in this repository.

Out of scope:

- Vulnerabilities in yt-dlp or FFmpeg themselves. Report those upstream: https://github.com/yt-dlp/yt-dlp and https://ffmpeg.org.
- Sites you download from. Getta does not control them.
- Social engineering and physical attacks.

## Supported versions

Only the latest release gets fixes. Older releases are not maintained.
