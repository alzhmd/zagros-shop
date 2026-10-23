<div align="center">

# Zagros — زاگرس

**A self-hosted Persian online store for home and refrigeration appliances — storefront, repair service, support desk and customer club in one small Rust binary.**

[![CI](https://github.com/alzhmd/zagros-shop/actions/workflows/ci.yml/badge.svg)](https://github.com/alzhmd/zagros-shop/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.88%2B-orange)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-336791)
![RTL](https://img.shields.io/badge/UI-Persian%20%2F%20RTL-green)

[فارسی](README.fa.md) · [Architecture](docs/ARCHITECTURE.md) · [Deployment](docs/DEPLOYMENT.md) · [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md)

<img src="docs/screenshots/01-home-desktop.png" alt="Zagros storefront — home page" width="860">

</div>

Zagros is a complete shop for a Persian-speaking market: customers browse and buy, order repairs, open tickets or chat live with an
operator, and earn points in a loyalty club; the owner runs everything from an admin panel. It is **server-rendered** (Axum + askama +
HTMX), keeps **every asset on its own server** (fonts, JavaScript — no CDN), and is tuned for a **small single-core machine** and for
connections where the international internet may drop: web server, background work and the Telegram bot run in **one process**.

## Features

**Storefront**
- Category tree, brands, **dynamic attribute filters**, price / stock / discount / rating filters, sorting, grid and list views.
- **Persian-aware search** on PostgreSQL (`pg_trgm` + `unaccent`) with live suggestions in the header.
- Product pages with **variants** (live price and stock), gallery, video embed, specification table, **reviews and ratings** (with a
  "verified purchase" badge and helpful votes), **questions and answers**, related products, recently viewed, wishlist.
- Compare-at prices, discount badges and deal end times.
- SEO: server-rendered HTML, `sitemap.xml`, `robots.txt`, Open Graph, schema.org (`Product`, `aggregateRating`).

**Accounts and checkout**
- Mobile-number login with **one-time codes** (OTP, argon2id), signed-cookie sessions, session management.
- Cart with live updates (HTMX + Alpine), address book, shipping methods, coupons, **loyalty points**, **wallet** and **gift cards** at checkout.
- Payment-gateway abstraction with **Zarinpal** and **Zibal**; cash on delivery; online payment can be switched off in the settings.
- Order timeline, re-order, customer-side cancellation, **returns**; stock is restored on cancellation or failed payment.

**After-sales**
- **Repair requests** (technician visit or drop-off), tracking by number, status history.
- **Support tickets** (department, priority) and **live chat over WebSocket** with an operator panel.
- **Customer club**: points earned automatically on purchase (idempotent), percentage/fixed coupons with caps and expiry, gift cards,
  notifications, back-in-stock alerts, and a **Telegram bot** that links to the customer's account (`/points`, `/help`).

**Admin panel**
- Full CRUD for products (variants, images, attributes), categories (tree) and brands.
- Orders, users (roles, blocking, manual points), reviews, Q&A, returns, repairs, tickets, live chats, coupons, gift cards, banners,
  broadcast notifications, and **site settings that apply without a restart**.

**Operations**
- One binary; migrations are applied on start; `GET /health`; Brotli/gzip compression and long-lived asset caching; a same-origin CSRF
  guard on form posts; in-memory TTL caches.
- Ready-made production files: a hardened **systemd unit** (no new privileges, read-only filesystem except `assets/`, memory caps) and an
  **nginx** configuration (TLS, HSTS, `nosniff`, rate limiting, WebSocket upgrade for the live chat).
- SMS through Kavenegar or SMS.ir (or a console mode for development) and e-mail through SMTP (console mode when unset).

> The interface is **Persian (RTL) only** today. Search uses `pg_trgm`; a Meilisearch adapter and a WebP/AVIF image pipeline are
> planned (see [the roadmap notes](docs/design-notes/ROADMAP.fa.md)).

## Screenshots

| Product list | Product page | Admin dashboard |
|:--:|:--:|:--:|
| <img src="docs/screenshots/03-products-desktop.png" width="280" alt="Product list with filters"> | <img src="docs/screenshots/02-product-desktop.png" width="280" alt="Product page"> | <img src="docs/screenshots/06-admin-dashboard.png" width="280" alt="Admin dashboard"> |

| Mobile — home | Mobile — product | Mobile — list | Admin — products |
|:--:|:--:|:--:|:--:|
| <img src="docs/screenshots/04-home-mobile.png" width="170" alt="Home on mobile"> | <img src="docs/screenshots/05-product-mobile.png" width="170" alt="Product on mobile"> | <img src="docs/screenshots/08-products-mobile.png" width="170" alt="List on mobile"> | <img src="docs/screenshots/07-admin-products.png" width="170" alt="Admin product management"> |

*Screenshots are taken from a local run with the seed data (`migrations/0003_seed.sql`). Product photos live in `assets/uploads/` and are not part of the repository.*

## Tech stack

| Layer | Choice |
|---|---|
| Language / runtime | Rust (edition 2021, **1.88+**), Tokio |
| Web | Axum 0.8, tower-http (compression, static files, limits, headers), WebSocket chat |
| Rendering | askama templates + HTMX + Alpine, self-hosted Vazirmatn font — server-side rendered, RTL |
| Database | PostgreSQL 16 through sqlx (runtime-checked queries, embedded migrations), `pg_trgm`, `unaccent`, `uuid-ossp` |
| Auth | argon2id, signed cookie sessions, OTP by SMS |
| Integrations | Zarinpal / Zibal, Kavenegar / SMS.ir, SMTP (lettre), Telegram Bot API (reqwest long polling) |

## Quick start

You need **Rust 1.88+** (`rustup`) and **Docker** (or any PostgreSQL 16 with the `pg_trgm`, `unaccent` and `uuid-ossp` extensions).

```bash
git clone https://github.com/alzhmd/zagros-shop.git && cd zagros-shop

docker compose up -d db      # PostgreSQL 16 with the app's built-in development credentials
cargo run -p web             # applies the migrations, then serves http://localhost:8080
```

That is the whole setup: the defaults in the code match `docker-compose.yml`, and the seed data gives you categories, brands and six
products to look at. Health check: `curl http://localhost:8080/health` → `ok`.

**Becoming the first admin.** Start the server once with your mobile number — you are promoted to `admin` on every start and then log
in with a one-time code. With the default `sms.provider = none` the code is printed in the server log instead of being sent:

```bash
ZAGROS__ADMIN__MOBILE=09123456789 cargo run -p web
# open http://localhost:8080/account/login → enter the number → read the code in the server log → then /admin
```

Port 5432 already taken? `ZAGROS_DB_PORT=5433 docker compose up -d db` and start the app with
`ZAGROS__DATABASE__URL=postgres://zagros:zagros_dev_2026@127.0.0.1:5433/zagros`.

## Configuration

Settings are read from `config/config.toml`, then `config/config.local.toml` (git-ignored — put secrets here), then **environment
variables**, which win. Environment names are `ZAGROS__<SECTION>__<KEY>`.

| Variable | Default | Meaning |
|---|---|---|
| `ZAGROS__SERVER__HOST` / `__PORT` | `0.0.0.0` / `8080` | listen address |
| `ZAGROS__SERVER__BASE_URL` | `http://localhost:8080` | public URL (links, gateway callbacks) |
| `ZAGROS__SERVER__COOKIE_SECRET` | built-in dev value | **set a random 64+ character value in production** (`openssl rand -hex 48`) |
| `ZAGROS__DATABASE__URL` | `postgres://zagros:zagros_dev_2026@127.0.0.1:5432/zagros` | PostgreSQL connection string |
| `ZAGROS__SMS__PROVIDER` | `none` | `none` (log) · `kavenegar` · `smsir` |
| `ZAGROS__SMS__API_KEY` / `__TEMPLATE` | empty | SMS provider key and OTP template |
| `ZAGROS__PAYMENT__ENABLED` | `false` | online payment on/off (cash on delivery stays available) |
| `ZAGROS__PAYMENT__PROVIDER` / `__MERCHANT` | `zibal` / `zibal` | `zarinpal` or `zibal`, merchant id (`zibal` is Zibal's test merchant) |
| `ZAGROS__TELEGRAM__ENABLED` / `__TOKEN` | `false` / empty | the club bot |
| `ZAGROS__EMAIL__SMTP_HOST`, `__SMTP_PORT` (587), `__SMTP_USER`, `__SMTP_PASS`, `__FROM` | empty → console mode | SMTP for order-confirmation and ticket-reply e-mails |
| `ZAGROS__ADMIN__MOBILE` | empty | mobile number promoted to admin at start |

> ⚠️ The development defaults (cookie secret, database password) are **public**. Never run a reachable instance with them — see
> [SECURITY.md](SECURITY.md) and [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Project layout

```
crates/
  web/       Axum app: routes, askama templates, auth, CSRF, uploads, mailer, WebSocket chat
  db/        sqlx queries and the embedded migrations runner (catalog, cart, orders, loyalty, accounts …)
  domain/    money (rial at the gateway, toman in the UI), shared errors
  payments/  PaymentGateway trait — Zarinpal, Zibal
  sms/       SmsSender trait — Kavenegar, SMS.ir, console
  bot/       Telegram bot (long polling over reqwest)
migrations/  19 ordered SQL files (schema + seed data)
assets/      CSS, fonts, JS (all local), uploads/
config/      default configuration
deploy/      systemd unit, nginx (TLS, WebSocket, rate limits), environment template
docs/        architecture, deployment, screenshots, design notes
scripts/     dev.sh helpers, screenshot script
```

## Development

```bash
./scripts/dev.sh run | build | release | check | test | fmt | clippy     # thin wrappers around cargo
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same checks, builds on the minimum supported Rust version and applies every migration to a fresh PostgreSQL 16.
`.cargo/config.toml` limits cargo to one parallel job (it was written for a one-core server); override with `CARGO_BUILD_JOBS=8`.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — crates, request flow, data model
- [Deployment](docs/DEPLOYMENT.md) (English) · [استقرار](docs/DEPLOYMENT.fa.md) (فارسی) — build, systemd, nginx + TLS, health checks
- [Design notes](docs/design-notes/PLAN.fa.md) and [roadmap](docs/design-notes/ROADMAP.fa.md) — the original Persian planning documents
- [Changelog](CHANGELOG.md) · [Security policy](SECURITY.md) · [Code of conduct](CODE_OF_CONDUCT.md)

## License

[MIT](LICENSE)
