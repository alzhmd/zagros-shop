# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The project is pre-1.0: configuration names and the database schema may still change between releases.

## [0.1.0] - 2026-10-24

First tagged release: the complete storefront, after-sales services and admin panel that had been developed in phases 0-6 and the
product-audit upgrade.

### Added
- **Storefront**: category tree, brands, dynamic attribute filters, price / stock / discount / rating filters, Persian-aware search
  (`pg_trgm` + `unaccent`) with live suggestions, product pages with variants, gallery, video embed, specification table, reviews and
  ratings (verified-purchase badge, helpful votes), questions and answers, related and recently viewed products, wishlist,
  compare-at prices with deal end times, banners.
- **SEO**: server-rendered HTML, `sitemap.xml`, `robots.txt`, Open Graph, schema.org `Product` / `aggregateRating`.
- **Accounts and checkout**: mobile-number login with one-time codes (argon2id), signed-cookie sessions with session management,
  live cart (HTMX + Alpine), address book, shipping methods, coupons, loyalty points, wallet and gift cards at checkout, Zarinpal and
  Zibal payment gateways, cash on delivery, order timeline, re-order, customer cancellation, returns, stock and points restored on
  cancellation or failed payment.
- **After-sales**: repair requests (technician visit or drop-off) with tracking and status history, support tickets, live chat over
  WebSocket with an operator panel.
- **Customer club**: automatic purchase points (idempotent), percentage / fixed coupons with caps, minimum order and expiry, gift
  cards, notifications, back-in-stock alerts, Telegram bot for account linking and `/points`.
- **Admin panel**: CRUD for products (variants, images, attributes), categories and brands; orders, users, reviews, Q&A, returns,
  repairs, tickets, live chats, coupons, gift cards, banners, broadcast notifications; site settings that apply without a restart.
- **Operations**: single binary with embedded migrations, `GET /health`, Brotli / gzip compression, same-origin CSRF guard, in-memory
  TTL caches, hardened systemd unit, nginx configurations (direct TLS and behind a CDN).
- **Integrations**: SMS through Kavenegar, SMS.ir or console mode; SMTP e-mail (console mode when unset).

### Changed
- The minimum supported Rust version is **1.88**. The manifest claimed 1.82, but the locked dependencies do not build on it; every
  crate now inherits `rust-version` from the workspace.
- The whole workspace is formatted with `rustfmt` and builds without clippy warnings (`-D warnings` in CI).
- Documentation: bilingual README, architecture guide, English and Persian deployment guides; the original Persian planning documents
  moved to `docs/design-notes/`.

### Fixed
- The deployment guide and the systemd unit told operators to set `DATABASE_URL`, which the application never reads. The variable is
  `ZAGROS__DATABASE__URL`; a deployment that followed the old text silently used the built-in development database URL.

### Infrastructure
- GitHub Actions CI: `rustfmt`, `clippy -D warnings`, tests, a build on the minimum supported Rust version, and every migration
  applied to a fresh PostgreSQL 16. Dependabot for cargo and Actions.
- `docker-compose.yml` for a one-command development database, `deploy/zagros.env.example` for production secrets.
- Issue and pull-request templates, contributing guide, code of conduct and security policy.

[0.1.0]: https://github.com/alzhmd/zagros-shop/releases/tag/v0.1.0
