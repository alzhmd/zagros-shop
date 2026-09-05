# استقرار زاگرس — جهان کولاک

راهنمای انتشار نسخه‌ی تولید. اپ یک باینری است (وب + ورکر + ربات تلگرام در یک پروسه).

## ۱) پیش‌نیازها روی سرور
- PostgreSQL 16 (نصب‌شده، دیتابیس و کاربر `zagros`).
- nginx (به‌عنوان reverse proxy / TLS).
- (اختیاری) دامنه + گواهی TLS ��ز certbot.

## ۲) بیلد نسخه‌ی تولید
روی یک ماشین با فضا/هسته‌ی کافی (نه لزوماً سرور):

```bash
cargo build --release -p web
# باینری: target/release/web
```

> **نکته‌ی باینری استاتیک (musl):** برای استقرار بدون وابستگی به glibc سرور:
> ```bash
> rustup target add x86_64-unknown-linux-musl
> cargo build --release --target x86_64-unknown-linux-musl -p web
> ```
> پروفایل release از قبل بهینه است (`lto="thin"`, `codegen-units=1`, `strip=true`, `panic="abort"`).

## ۳) چیدمان فایل‌ها روی سرور (`/opt/zagros`)
```
/opt/zagros/
├─ web                 # باینری منتشرشده
├─ assets/             # CSS، فونت، JS (همه لوکال، بدون CDN)
├─ config/
│  └─ config.toml      # تنظیمات غیرمحرمانه
├─ migrations/         # SQL — هنگام اجرا خودکار اعمال می‌شود
└─ zagros.env          # رازها (خارج از مخزن) — مرحله‌ی بعد
```

```bash
sudo mkdir -p /opt/zagros
sudo useradd -r -s /usr/sbin/nologin zagros 2>/dev/null || true
sudo cp target/release/web /opt/zagros/
sudo cp -r assets config migrations /opt/zagros/
sudo chown -R zagros:zagros /opt/zagros
```

## ۴) فایل محیطی رازها (`/opt/zagros/zagros.env`)
```ini
DATABASE_URL=postgres://zagros:CHANGE_ME@127.0.0.1:5432/zagros
# secret کوکی: یک رشته‌ی تصادفی ۶۴+ کاراکتری (مثلاً: openssl rand -hex 48)
ZAGROS__SERVER__COOKIE_SECRET=CHANGE_ME_LONG_RANDOM
ZAGROS__SERVER__BASE_URL=https://zagros.example.ir
# درگاه پرداخت (هنگام فعال‌سازی):
ZAGROS__PAYMENT__ENABLED=true
ZAGROS__PAYMENT__PROVIDER=zarinpal
ZAGROS__PAYMENT__MERCHANT=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
# پیامک (هنگام فعال‌سازی):
ZAGROS__SMS__PROVIDER=kavenegar
ZAGROS__SMS__API_KEY=xxxxx
ZAGROS__SMS__TEMPLATE=zagros-otp
# ربات تلگرام:
ZAGROS__TELEGRAM__ENABLED=true
ZAGROS__TELEGRAM__TOKEN=xxxxxxxxx:yyyyyyyyyyyyyyyyyyyy
```
```bash
sudo chown zagros:zagros /opt/zagros/zagros.env
sudo chmod 600 /opt/zagros/zagros.env
```

## ۵) سرویس systemd
```bash
sudo cp deploy/zagros.service /etc/systemd/system/zagros.service
sudo systemctl daemon-reload
sudo systemctl enable --now zagros
sudo systemctl status zagros
journalctl -u zagros -f      # مشاهده‌ی لاگ + تأیید «سرور زاگرس بالا آمد» و «ربات تلگرام متصل شد»
```

## ۶) nginx + TLS
```bash
sudo cp deploy/nginx.conf /etc/nginx/sites-available/zagros
sudo ln -s /etc/nginx/sites-available/zagros /etc/nginx/sites-enabled/
# دامنه و مسیر گواهی را در فایل ویرایش کنید، سپس:
sudo certbot --nginx -d zagros.example.ir -d www.zagros.example.ir   # یا گواهی دستی
sudo nginx -t && sudo systemctl reload nginx
```

## ۷) بررسی سلامت پس از استقرار
- `curl -fsS https://دامنه/health` → `ok`
- صفحه‌ی اصلی، یک صفحه‌ی محصول، و `/sitemap.xml` و `/robots.txt` باز شوند.
- چت زنده: یک گفت‌وگو شروع کنید و ارسال پیام را بسنجید (WebSocket از پشت nginx).
- پاسخ‌ها فشرده باشند: `curl -H 'Accept-Encoding: br' -I https://دامنه/` → `content-encoding: br`.
- ربات: در تلگرام `/help` بزنید.

## موارد مؤجل (هنگام افزایش فضای دیسک)
- **Meilisearch:** جستجو فعلاً با pg_trgm کار می‌کند؛ آداپتور پشت لایه‌ی catalog قابل‌تعویض است.
- **خط لوله‌ی تصویر WebP/AVIF:** فعلاً تصاویر با مسیر سرو می‌شوند؛ افزودن `image`/`ravif` به فضای دیسک نیاز دارد.
