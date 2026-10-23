<div dir="rtl" align="center">

# زاگرس — جهان کولاک

**فروشگاه آنلاین خودمیزبان برای لوازم خانگی و برودتی — فروشگاه، خدمات تعمیرات، پشتیبانی و باشگاه مشتریان در یک باینری سبک Rust.**

[![CI](https://github.com/alzhmd/zagros-shop/actions/workflows/ci.yml/badge.svg)](https://github.com/alzhmd/zagros-shop/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.88%2B-orange)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-336791)
![RTL](https://img.shields.io/badge/UI-Persian%20%2F%20RTL-green)

[English](README.md) · [معماری](docs/ARCHITECTURE.md) · [استقرار](docs/DEPLOYMENT.fa.md) · [تغییرات](CHANGELOG.md) · [مشارکت](CONTRIBUTING.md)

<img src="docs/screenshots/01-home-desktop.png" alt="فروشگاه زاگرس — صفحهٔ اصلی" width="860">

</div>

<div dir="rtl">

زاگرس یک فروشگاه کامل برای بازار فارسی‌زبان است: مشتری مرور و خرید می‌کند، درخواست تعمیر ثبت می‌کند، تیکت می‌زند یا با اپراتور چت زنده
دارد و در باشگاه مشتریان امتیاز می‌گیرد؛ صاحب فروشگاه همه‌چیز را از پنل مدیریت اداره می‌کند. برنامه **سمت سرور رندر می‌شود**
(Axum + askama + HTMX)، **همهٔ فایل‌هایش را از سرور خودش می‌دهد** (فونت و جاوااسکریپت — بدون CDN) و برای **یک سرور کوچک تک‌هسته** و
شرایطی که اینترنت بین‌الملل ممکن است قطع شود بهینه شده است: وب‌سرور، کارهای پس‌زمینه و ربات تلگرام همه در **یک پروسه** هستند.

## امکانات

**فروشگاه**
- درخت دسته‌بندی، برندها، **فیلترهای پویای ویژگی**، فیلتر قیمت / موجودی / تخفیف / امتیاز، مرتب‌سازی، نمای شبکه‌ای و فهرستی.
- **جستجوی فارسی** روی PostgreSQL (`pg_trgm` + `unaccent`) با پیشنهاد زنده در هدر.
- صفحهٔ محصول با **تنوع‌ها** (قیمت و موجودی زنده)، گالری، ویدئو، جدول مشخصات، **نقد و امتیاز** (نشان «خرید تأییدشده» و رأی «مفید بود»)،
  **پرسش و پاسخ**، محصولات مرتبط، بازدیدهای اخیر و علاقه‌مندی‌ها.
- قیمت قبل از تخفیف، نشان درصد تخفیف و زمان پایان پیشنهاد ویژه.
- سئو: HTML سمت سرور، `sitemap.xml`، `robots.txt`، Open Graph و schema.org (`Product`، `aggregateRating`).

**حساب و تسویه**
- ورود با شمارهٔ موبایل و **کد یک‌بارمصرف** (OTP با argon2id)، نشست با کوکی امضاشده، مدیریت نشست‌ها.
- سبد خرید زنده (HTMX + Alpine)، دفترچهٔ آدرس، روش‌های ارسال، کد تخفیف، **امتیاز باشگاه**، **کیف پول** و **کارت هدیه** هنگام تسویه.
- انتزاع درگاه پرداخت با **زرین‌پال** و **زیبال**؛ پرداخت در محل؛ پرداخت آنلاین از تنظیمات قابل خاموش‌کردن است.
- خط زمانی سفارش، سفارش مجدد، لغو توسط مشتری، **مرجوعی**؛ با لغو یا شکست پرداخت موجودی برمی‌گردد.

**خدمات پس از فروش**
- **درخواست تعمیر** (اعزام کارشناس یا تحویل به انبار)، پیگیری با شماره و تاریخچهٔ وضعیت.
- **تیکت پشتیبانی** (بخش و اولویت) و **چت زندهٔ WebSocket** با پنل اپراتور.
- **باشگاه مشتریان**: کسب خودکار امتیاز با خرید (idempotent)، کد تخفیف درصدی/ثابت با سقف و انقضا، کارت هدیه، اعلان‌ها، هشدار موجود‌شدن،
  و یک **ربات تلگرام** که به حساب مشتری وصل می‌شود (`/points`، `/help`).

**پنل مدیریت**
- مدیریت کامل (CRUD) محصولات (تنوع، تصویر، ویژگی)، دسته‌بندی‌ها (درختی) و برندها.
- سفارش‌ها، کاربران (نقش، مسدودسازی، امتیاز دستی)، نظرات، پرسش و پاسخ، مرجوعی‌ها، تعمیرات، تیکت‌ها، گفت‌وگوها، کد تخفیف، کارت هدیه،
  بنرها، اعلان همگانی و **تنظیمات سایت که بدون ری‌استارت اعمال می‌شوند**.

**عملیات**
- یک باینری؛ migrationها هنگام اجرا اعمال می‌شوند؛ `GET /health`؛ فشرده‌سازی Brotli/gzip و کش طولانی asset؛ محافظ CSRF هم‌مبدأ
  روی فرم‌ها؛ کش درون‌حافظه‌ای با TTL.
- فایل‌های آمادهٔ تولید: **یونیت systemd** سخت‌سازی‌شده (بدون امتیاز اضافه، فایل‌سیستم فقط‌خواندنی جز `assets/`، سقف حافظه) و پیکربندی
  **nginx** (TLS، HSTS، `nosniff`، محدودیت نرخ، WebSocket برای چت زنده).
- پیامک با کاوه‌نگار یا SMS.ir (یا حالت کنسول برای توسعه) و ایمیل با SMTP (در نبود تنظیم، حالت کنسول).

> رابط کاربری فعلاً **فقط فارسی (راست‌به‌چپ)** است. جستجو با `pg_trgm` انجام می‌شود؛ آداپتور Meilisearch و خط لولهٔ تصویر WebP/AVIF
> برنامه‌ریزی شده‌اند ([یادداشت‌های نقشهٔ راه](docs/design-notes/ROADMAP.fa.md)).

## تصاویر

| فهرست محصولات | صفحهٔ محصول | داشبورد مدیریت |
|:--:|:--:|:--:|
| <img src="docs/screenshots/03-products-desktop.png" width="280" alt="فهرست محصولات با فیلتر"> | <img src="docs/screenshots/02-product-desktop.png" width="280" alt="صفحهٔ محصول"> | <img src="docs/screenshots/06-admin-dashboard.png" width="280" alt="داشبورد مدیریت"> |

| موبایل — خانه | موبایل — محصول | موبایل — فهرست | مدیریت — محصولات |
|:--:|:--:|:--:|:--:|
| <img src="docs/screenshots/04-home-mobile.png" width="170" alt="خانه در موبایل"> | <img src="docs/screenshots/05-product-mobile.png" width="170" alt="محصول در موبایل"> | <img src="docs/screenshots/08-products-mobile.png" width="170" alt="فهرست در موبایل"> | <img src="docs/screenshots/07-admin-products.png" width="170" alt="مدیریت محصولات"> |

*تصاویر از اجرای محلی با دادهٔ نمونه (`migrations/0003_seed.sql`) گرفته شده‌اند. عکس محصولات در `assets/uploads/` نگه‌داری می‌شود و در مخزن نیست.*

## استک فنی

| لایه | انتخاب |
|---|---|
| زبان / اجرا | Rust (نسخهٔ ۲۰۲۱، **۱.۸۸ به بالا**)، Tokio |
| وب | Axum 0.8، tower-http (فشرده‌سازی، فایل ایستا، محدودیت‌ها)، چت WebSocket |
| رندر | قالب askama + HTMX + Alpine، فونت خودمیزبان وزیرمتن — رندر سمت سرور، RTL |
| پایگاه داده | PostgreSQL 16 با sqlx (کوئری‌های بررسی‌شده در زمان اجرا، migrationهای توکار)، `pg_trgm`، `unaccent`، `uuid-ossp` |
| احراز هویت | argon2id، نشست با کوکی امضاشده، OTP با پیامک |
| یکپارچه‌سازی | زرین‌پال / زیبال، کاوه‌نگار / SMS.ir، SMTP (lettre)، Telegram Bot API (reqwest با long polling) |

## شروع سریع

به **Rust 1.88 به بالا** (`rustup`) و **Docker** نیاز دارید (یا هر PostgreSQL 16 با extensionهای `pg_trgm`، `unaccent` و `uuid-ossp`).

</div>

```bash
git clone https://github.com/alzhmd/zagros-shop.git && cd zagros-shop

docker compose up -d db      # PostgreSQL 16 با همان مشخصات پیش‌فرض توسعهٔ برنامه
cargo run -p web             # migrationها را اعمال و http://localhost:8080 را بالا می‌آورد
```

<div dir="rtl">

همین کافی است: مقدارهای پیش‌فرض در کد با `docker-compose.yml` یکی است و دادهٔ نمونه چند دسته، برند و شش محصول می‌دهد.
بررسی سلامت: `curl http://localhost:8080/health` ← `ok`.

**اولین مدیر.** یک بار سرور را با شمارهٔ موبایل خودتان اجرا کنید؛ آن شماره در هر راه‌اندازی به `admin` ارتقا می‌یابد و با کد
یک‌بارمصرف وارد می‌شود. با `sms.provider = none` (پیش‌فرض) کد به‌جای ارسال، در لاگ سرور چاپ می‌شود:

</div>

```bash
ZAGROS__ADMIN__MOBILE=09123456789 cargo run -p web
# http://localhost:8080/account/login را باز کنید ← شماره را بزنید ← کد را از لاگ سرور بخوانید ← سپس /admin
```

<div dir="rtl">

پورت ۵۴۳۲ اشغال است؟ `ZAGROS_DB_PORT=5433 docker compose up -d db` و برنامه را با
`ZAGROS__DATABASE__URL=postgres://zagros:zagros_dev_2026@127.0.0.1:5433/zagros` اجرا کنید.

## پیکربندی

تنظیمات به‌ترتیب از `config/config.toml`، سپس `config/config.local.toml` (در git نیست — رازها را اینجا بگذارید) و در پایان از
**متغیرهای محیطی** خوانده می‌شود که اولویت دارند. نام متغیرها `ZAGROS__<بخش>__<کلید>` است.

| متغیر | پیش‌فرض | معنی |
|---|---|---|
| `ZAGROS__SERVER__HOST` / `__PORT` | `0.0.0.0` / `8080` | نشانی شنود |
| `ZAGROS__SERVER__BASE_URL` | `http://localhost:8080` | نشانی عمومی (لینک‌ها و بازگشت درگاه) |
| `ZAGROS__SERVER__COOKIE_SECRET` | مقدار توسعهٔ داخلی | **در تولید یک رشتهٔ تصادفی ۶۴+ کاراکتری بگذارید** (`openssl rand -hex 48`) |
| `ZAGROS__DATABASE__URL` | `postgres://zagros:zagros_dev_2026@127.0.0.1:5432/zagros` | رشتهٔ اتصال PostgreSQL |
| `ZAGROS__SMS__PROVIDER` | `none` | `none` (لاگ) · `kavenegar` · `smsir` |
| `ZAGROS__SMS__API_KEY` / `__TEMPLATE` | خالی | کلید ارائه‌دهندهٔ پیامک و قالب OTP |
| `ZAGROS__PAYMENT__ENABLED` | `false` | روشن/خاموش بودن پرداخت آنلاین (پرداخت در محل می‌ماند) |
| `ZAGROS__PAYMENT__PROVIDER` / `__MERCHANT` | `zibal` / `zibal` | `zarinpal` یا `zibal` و شناسهٔ پذیرنده (`zibal` مرچنت آزمایشی زیبال است) |
| `ZAGROS__TELEGRAM__ENABLED` / `__TOKEN` | `false` / خالی | ربات باشگاه مشتریان |
| `ZAGROS__EMAIL__SMTP_HOST`، `__SMTP_PORT` (587)، `__SMTP_USER`، `__SMTP_PASS`، `__FROM` | خالی ← حالت کنسول | SMTP برای ایمیل تأیید سفارش و پاسخ تیکت |
| `ZAGROS__ADMIN__MOBILE` | خالی | شمارهٔ موبایلی که هنگام اجرا به مدیر ارتقا می‌یابد |

> ⚠️ مقدارهای پیش‌فرض توسعه (راز کوکی و رمز پایگاه داده) **عمومی** هستند. هرگز نمونه‌ای را که از بیرون در دسترس است با آن‌ها اجرا نکنید —
> [SECURITY.md](SECURITY.md) و [docs/DEPLOYMENT.fa.md](docs/DEPLOYMENT.fa.md) را ببینید.

## ساختار پروژه

</div>

```
crates/
  web/       اپ Axum: مسیرها، قالب‌های askama، احراز هویت، CSRF، آپلود، ایمیل، چت WebSocket
  db/        کوئری‌های sqlx و اجرای migrationهای توکار (کاتالوگ، سبد، سفارش، باشگاه، حساب …)
  domain/    پول (ریال در مرز درگاه، تومان در UI)، خطاهای مشترک
  payments/  trait ـ PaymentGateway — زرین‌پال، زیبال
  sms/       trait ـ SmsSender — کاوه‌نگار، SMS.ir، کنسول
  bot/       ربات تلگرام (long polling با reqwest)
migrations/  ۱۹ فایل SQL مرتب (ساختار + دادهٔ نمونه)
assets/      CSS، فونت، JS (همه محلی)، uploads/
config/      پیکربندی پیش‌فرض
deploy/      یونیت systemd، nginx (TLS، WebSocket، محدودیت نرخ)، الگوی فایل محیطی
docs/        معماری، استقرار، تصاویر، یادداشت‌های طراحی
scripts/     dev.sh، اسکریپت اسکرین‌شات
```

<div dir="rtl">

## توسعه

</div>

```bash
./scripts/dev.sh run | build | release | check | test | fmt | clippy     # پوشش نازک روی cargo
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

<div dir="rtl">

CI همین بررسی‌ها را اجرا می‌کند، روی کمترین نسخهٔ Rust پشتیبانی‌شده بیلد می‌گیرد و همهٔ migrationها را روی یک PostgreSQL 16 تازه اعمال
می‌کند. `.cargo/config.toml` کارگو را به یک کار هم‌زمان محدود می‌کند (برای سرور تک‌هسته نوشته شده)؛ با `CARGO_BUILD_JOBS=8` تغییر دهید.

## مستندات

- [معماری](docs/ARCHITECTURE.md) — crateها، جریان درخواست، مدل داده (انگلیسی)
- [استقرار](docs/DEPLOYMENT.fa.md) · [Deployment](docs/DEPLOYMENT.md) — بیلد، systemd، nginx و TLS، بررسی سلامت
- [یادداشت‌های طراحی](docs/design-notes/PLAN.fa.md) و [نقشهٔ راه](docs/design-notes/ROADMAP.fa.md) — اسناد برنامه‌ریزی اولیه
- [تغییرات](CHANGELOG.md) · [سیاست امنیتی](SECURITY.md) · [آیین‌نامهٔ رفتار](CODE_OF_CONDUCT.md)

## مجوز

[MIT](LICENSE)

</div>
