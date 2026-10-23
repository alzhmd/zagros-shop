# Security policy

## Reporting a vulnerability

Please report security problems **privately** through
[GitHub security advisories](https://github.com/alzhmd/zagros-shop/security/advisories/new) instead of a public issue.
Include what you found, how to reproduce it and, if you can, a suggested fix. You will get an answer within a few days.

## What is sensitive

* `ZAGROS__SERVER__COOKIE_SECRET` — signs the session cookies. Anyone who knows it can forge a session, including an admin one.
  **The fallback value built into the source is public** and exists for local development only: always set your own random value
  (64+ characters, `openssl rand -hex 48`) before a deployment is reachable from the internet.
* `ZAGROS__DATABASE__URL` — the PostgreSQL password. The development default (`zagros_dev_2026`) is public too.
* Payment merchant id, SMS provider key, Telegram bot token and SMTP credentials.
* `zagros.env` (see `deploy/zagros.env.example`) and `config/config.local.toml` hold these values. Keep them out of the repository
  (both are git-ignored or live outside it), readable only by the service user (`chmod 600`).
* Uploaded product images in `assets/uploads/` are public by design; nothing else under `assets/` should hold private data.

## How the application protects accounts

* Login codes are random, stored **hashed** (argon2id), expire, are single-use and count failed attempts.
* Session cookies are signed and `SameSite=Lax`; state-changing requests are additionally checked for a same-origin `Origin`/`Referer`.
* Admin pages require the `admin` role; the first administrator is promoted from `ZAGROS__ADMIN__MOBILE` at start-up, so restrict who
  can edit the service environment.
* Run the app behind nginx with the provided configuration (TLS, HSTS, rate limiting).

## Supported versions

Only the latest release (and `main`) receives fixes.
