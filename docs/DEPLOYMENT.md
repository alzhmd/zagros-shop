# Deploying Zagros

[فارسی](DEPLOYMENT.fa.md)

A production release is **one binary** (web server, background work and the Telegram bot in one process) plus the `assets/` and
`config/` folders, behind nginx, with PostgreSQL 16.

## 1. Prerequisites on the server

- PostgreSQL 16, nginx, and (optionally) a domain with a TLS certificate from certbot.
- A database and role for the app. Run once as the `postgres` user:

  ```bash
  sudo -u postgres psql -c "CREATE USER zagros WITH PASSWORD 'CHANGE_ME';"
  sudo -u postgres psql -c "CREATE DATABASE zagros OWNER zagros;"
  ```

  The migrations create the extensions they need (`pg_trgm`, `unaccent`, `uuid-ossp` — all *trusted*, so the ordinary `zagros` role can do
  it). If your policy forbids that, create them yourself as a superuser: `psql -d zagros -c 'CREATE EXTENSION pg_trgm; CREATE EXTENSION unaccent; CREATE EXTENSION "uuid-ossp";'`.

## 2. Build

Build on any machine with enough cores and RAM — not necessarily the server. You need **Rust 1.88+**.

```bash
CARGO_BUILD_JOBS=8 cargo build --release -p web      # → target/release/web
```

`.cargo/config.toml` pins cargo to one job (it was written for a one-core server), hence the override above. For a binary with no glibc
dependency use the musl target:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl -p web
```

The release profile is already tuned (`lto = "thin"`, `codegen-units = 1`, `strip`, `panic = "abort"`).

## 3. Layout on the server (`/opt/zagros`)

```
/opt/zagros/
├─ web                 # the release binary
├─ assets/             # CSS, fonts, JS (all local, no CDN); uploads/ is writable by the service user
├─ config/
│  └─ config.toml      # non-secret defaults
└─ zagros.env          # secrets — outside the repository, mode 600
```

```bash
sudo mkdir -p /opt/zagros
sudo useradd -r -s /usr/sbin/nologin zagros 2>/dev/null || true
sudo cp target/release/web /opt/zagros/
sudo cp -r assets config /opt/zagros/
sudo chown -R zagros:zagros /opt/zagros
```

The SQL migrations are **embedded in the binary** and applied automatically at start-up; the `migrations/` folder is not read at
runtime.

## 4. Secrets file (`/opt/zagros/zagros.env`)

Start from the template and fill it in:

```bash
sudo cp deploy/zagros.env.example /opt/zagros/zagros.env
sudo chown zagros:zagros /opt/zagros/zagros.env && sudo chmod 600 /opt/zagros/zagros.env
```

At minimum set these — note the names, they are `ZAGROS__<SECTION>__<KEY>`:

```ini
ZAGROS__DATABASE__URL=postgres://zagros:CHANGE_ME@127.0.0.1:5432/zagros
ZAGROS__SERVER__COOKIE_SECRET=<openssl rand -hex 48>
ZAGROS__SERVER__BASE_URL=https://your-domain.example
```

> ⚠️ **The built-in cookie secret and database password are public** (they are in the source). If you forget to override them the app
> still starts — with a publicly known key for signing sessions. Always set `ZAGROS__SERVER__COOKIE_SECRET`. A plain `DATABASE_URL`
> variable is **not** read; use `ZAGROS__DATABASE__URL`.

Payment, SMS, Telegram and e-mail settings are optional and listed in the template and in the README.

## 5. systemd

```bash
sudo cp deploy/zagros.service /etc/systemd/system/zagros.service
sudo systemctl daemon-reload
sudo systemctl enable --now zagros
sudo systemctl status zagros
journalctl -u zagros -f        # look for "سرور زاگرس … بالا آمد" (and the Telegram bot line when it is enabled)
```

The unit runs as the unprivileged `zagros` user with `NoNewPrivileges`, a private `/tmp`, a read-only filesystem except
`/opt/zagros/assets`, and memory limits (`MemoryHigh=768M`, `MemoryMax=1G`).

## 6. nginx and TLS

```bash
sudo cp deploy/nginx.conf /etc/nginx/sites-available/zagros
sudo ln -s /etc/nginx/sites-available/zagros /etc/nginx/sites-enabled/
# edit the server_name and certificate paths, then:
sudo certbot --nginx -d your-domain.example -d www.your-domain.example     # or install a certificate by hand
sudo nginx -t && sudo systemctl reload nginx
```

`deploy/nginx.conf` terminates TLS, adds HSTS and `nosniff`, rate-limits requests (10 req/s per IP), upgrades WebSocket connections for the
live chat and allows uploads up to 12 MB. `deploy/nginx-arvan.conf` is a variant for sites behind the ArvanCloud CDN: the CDN
terminates TLS at the edge and talks to the origin over plain HTTP on port 80.

## 7. Checks after deploying

- `curl -fsS https://your-domain.example/health` → `ok`
- The home page, a product page, `/sitemap.xml` and `/robots.txt` load.
- Live chat: start a conversation and send a message (this exercises the WebSocket through nginx).
- Responses are compressed: `curl -H 'Accept-Encoding: br' -I https://your-domain.example/` → `content-encoding: br`.
- Telegram bot (if enabled): send `/help` to it.
- Log in with the number you set in `ZAGROS__ADMIN__MOBILE` and open `/admin`.

## Updating

```bash
git pull && CARGO_BUILD_JOBS=8 cargo build --release -p web
sudo systemctl stop zagros && sudo cp target/release/web /opt/zagros/web && sudo rsync -a assets/ /opt/zagros/assets/ --exclude uploads
sudo systemctl start zagros
```

New migrations run automatically on the next start. Back up the database (`pg_dump -Fc zagros`) and `assets/uploads/` before upgrading.

## Postponed

- **Meilisearch** — search currently runs on PostgreSQL (`pg_trgm`); an adapter can sit behind the catalogue layer later.
- **WebP/AVIF image pipeline** — uploaded images are served as they are.
