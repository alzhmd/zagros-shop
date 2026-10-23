# Contributing

Thanks for helping! Bug reports, ideas, translations and pull requests are all welcome.

## Set up

```bash
git clone https://github.com/alzhmd/zagros-shop.git && cd zagros-shop
docker compose up -d db          # PostgreSQL 16 with the development credentials
cargo run -p web                 # migrations run on start; http://localhost:8080
```

Rust **1.88+** is required. Before you push, run the same checks as CI:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`.cargo/config.toml` limits cargo to one job (it was written for a one-core server); use `CARGO_BUILD_JOBS=8` for faster builds.

## Ground rules

* **Migrations are append-only.** Never edit a migration that has been released; add the next numbered file in `migrations/`
  (`0020_….sql`). Make it work on a fresh database *and* on the previous schema — CI applies every file in order.
* **Stay self-hosted.** No CDN, no third-party scripts or fonts; assets live in `assets/`. The site has to keep working when the
  international internet does not.
* **Money is integer rial** at the payment boundary and in the database; toman is a presentation concern (`crates/domain/src/money.rs`).
* **Persian, right-to-left UI.** Check new pages in RTL and on a narrow screen; keep user-facing text Persian.
* New behaviour comes with a test where it can be unit-tested (see the existing tests in `crates/db`, `crates/domain` and `crates/web`).
* Never commit secrets, tokens, `.env` files, uploaded images or real customer data. Use neutral examples.
* Update `CHANGELOG.md` for user-visible changes and the docs when behaviour or configuration changes.

## Pull requests

Small, focused PRs are reviewed fastest. Describe the problem, the approach and how you tested it. Be kind: this project follows the
[Code of Conduct](CODE_OF_CONDUCT.md). Security problems go through the [security policy](SECURITY.md), not a public issue.
