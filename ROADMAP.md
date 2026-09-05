# نقشهٔ راه نهایی‌سازی زاگرس / جهان کولاک

> وضعیت پایه: فاز ۰–۶ + فاز ارتقا انجام‌شده، سرور up (health 200)، صفر warning، دیسک ۴.۱G آزاد، تک‌هسته، بدون git.
> اصول ثابت: ۱۰۰٪ خودمیزبان، بدون CDN/JS خارجی، RTL فارسی، پول=ریال در مرز/تومان در UI، پرفورمنس بالا، WCAG AA، بدون افزودن crate سنگین جدید.

---

## خلأهای کلان (سیستم‌های غایب)
1. قیمت‌گذاری تخفیف: محصول/تنوع `compare_at_price` ندارد → نه قیمت قبل، نه درصد تخفیف، نه تایمر شگفت‌انگیز.
2. کیف پول (wallet) کاملاً غایب.
3. اعلان‌ها (notifications) کاملاً غایب.
4. کارت هدیه (gift card) غایب.
5. پرسش‌وپاسخ محصول (Q&A) غایب.
6. بنر/کمپین مدیریت‌پذیر (اسلایدر خانه) غایب.
7. سطوح باشگاه (برنزی/نقره‌ای/طلایی) غایب.
8. هشدار موجود‌شدن / کاهش قیمت، ذخیره برای بعد غایب.
9. فاکتور PDF، صفحهٔ دسته‌بندی‌ها، تب‌بار موبایل غایب.
10. فیلدهای پروفایل (تولد/جنسیت/حذف حساب)، روش‌های ارسال غایب.
11. CSRF روی فرم‌های POST غایب (ریسک تولید).
12. بوت‌استرپ ادمین اول وجود ندارد (set_role نیازمند ادمین موجود).

---

## Migrationهای جدید (۰۰۱۱+)
- `0011_pricing_deals` — `product_variants.compare_at_price bigint`, `products.deal_ends_at timestamptz`, index.
- `0012_wallet` — `users.wallet_balance bigint default 0` + `wallet_transactions(user_id,delta,kind,ref_order,note,created_at)`.
- `0013_notifications` — `notifications(id,user_id,kind,title,body,url,read_at,created_at)` + index user/unread.
- `0014_gift_cards` — `gift_cards(code,initial_rial,balance_rial,expires_at,issued_to,redeemed_by,created_at)`.
- `0015_qa` — `product_questions(id,product_id,user_id,author,body,status,created_at)` + `product_answers(id,question_id,body,is_staff,created_at)`.
- `0016_banners` — `banners(id,placement,title,subtitle,image_path,link,sort_order,is_active,starts_at,ends_at)`.
- `0017_alerts_saved` — `stock_alerts(user_id,variant_id,created_at,notified_at)`, `cart_items.saved_for_later bool`.
- `0018_profile_shipping` — `users.birthdate date`, `users.gender text`, `orders.shipping_method text`, settings سطوح/ارسال.
- سطوح باشگاه: بدون جدول، از آستانهٔ settings محاسبه می‌شود.

---

## U1 — پایه و موانع انتشار  ⛔ launch-blocker
- بوت‌استرپ ادمین: در main، اگر `ZAGROS__ADMIN__MOBILE` ست بود → upsert+promote به admin هنگام استارت. (main.rs، db/accounts).
- CSRF: middleware توکن per-session (کوکی امضاشده + hidden `_csrf`)، اعتبارسنجی روی همهٔ POST؛ helper در askama. (web.rs/auth.rs، همهٔ فرم‌ها).
- مدل تخفیف: `compare_at_price` در variant؛ helperهای `discount_pct()`/`old_price_fa()` در models؛ نمایش قیمت‌خط‌خورده + بَج «٪…» در کارت/محصول/سبد/تسویه. (0011، models.rs، catalog CARD_SELECT، product.html، catalog.html، home.html، offers.html).
- روش‌های ارسال: گزینه‌های settings (عادی/پیشتاز/فوری + هزینه + زمان تقریبی)، انتخاب در تسویه، ذخیره در order. (0018، checkout.rs/html، settings).
- JSON-LD offer: افزودن `priceValidUntil`, `availability` درست + قیمت تخفیفی. (product.html).

## U2 — ویترین خانه + بنر + دسته‌ها + تب‌بار
- بنر/کمپین (admin CRUD): اسلایدر hero خانه، بنر ۲×۲ برند/کمپین، نوار کد تخفیف. (0016، routes/admin_banners.rs، home.rs/html، admin nav).
- اسلایدر خودمیزبان: CSS scroll-snap + Alpine، بدون lib؛ کنترل نقطه/کیبورد، lazy. (home.html، app.css).
- تایمر شگفت‌انگیز: ریل «پیشنهاد لحظه‌ای» محصولات `deal_ends_at`، شمارش معکوس JS سبک. (home.rs/html، app.js).
- ریل‌های خانه: دسته‌های آیکونی دایره‌ای، پربازدیدها، بازدیدهای اخیر (کوکی)، جدیدترین‌ها، بنر کد تخفیف. (home.rs/html).
- دکمه‌های میانبر: ارسال فوری/تخفیف داغ/دسته‌ها (chips زیر هدر). (home.html).
- صفحهٔ `/categories`: گرید آیکونی + آکاردئون زیردسته + پرفروش‌های هر دسته + برندهای محبوب. (routes/catalog.rs یا pages، categories.html، روت).
- تب‌بار موبایل ثابت: خانه/دسته‌ها/سبد(badge)/پروفایل — partial + CSS، فقط ≤768px. (base.html، partials، app.css).

## U3 — کیف پول
- مدل: balance + دفتر تراکنش (آینهٔ loyalty). (0012، db/wallet.rs).
- شارژ: top-up آنلاین از طریق فلوی پرداخت موجود (order نوع wallet_topup) — تا فعال‌شدن درگاه، فقط شارژ دستی ادمین + بازگشتی‌ها. (checkout.rs، admin).
- پرداخت با کیف پول در تسویه (مثل امتیاز، تا سقف موجودی). (checkout.rs/html).
- بازگشت خودکار به کیف پول: لغو سفارش، پرداخت ناموفق، تأیید مرجوعی → credit. (checkout.rs، returns، admin_returns).
- UI حساب: موجودی، تاریخچهٔ واریز/برداشت با فیلتر تاریخ، دکمهٔ شارژ. (account.rs، account/wallet.html، sidebar).

## U4 — اعلان‌ها + هشدارها
- اعلان DB + زنگ هدر با شمار unread (HTMX load/poll) + صفحهٔ `/account/notifications` + علامت‌خوانده. (0013، db/notifications.rs، account.rs، partials/header، base.html).
- محرک‌ها: تغییر وضعیت سفارش، تأیید نقد، کد تخفیف اختصاصی، باشگاه. (نقاط مربوط: admin_orders، admin_reviews، loyalty_hook).
- هشدار موجود‌شدن: دکمه در محصول ناموجود → ثبت؛ هنگام stock>0 (admin) → notify. (0017، product.html، admin_products).
- کاهش قیمت: هنگام افت قیمت تنوع نسبت به قبل → notify کاربران wishlist. (admin_products، wishlist).

## U5 — عمق حساب و باشگاه
- پروفایل: افزودن تولد/جنسیت؛ حذف حساب (soft: is_active=false + ناشناس‌سازی)؛ احراز فقط OTP (بدون رمز). (0018، account.rs/profile.html).
- کارت هدیه: صدور خودکار پس از عضویت (settings تاگل+مبلغ+انقضا)؛ ورود کد → credit کیف پول؛ نمایش موجودی/انقضا. (0014، db/gift_cards.rs، account، signup hook در verify_otp).
- سطوح باشگاه: محاسبه از آستانهٔ settings، نمایش سطح/پیشرفت/مزایا/«چطور امتیاز بگیریم». (account/club.html، state settings).
- ذخیره برای بعد: تاگل در سبد، خارج از جمع. (0017، cart.rs، cart_body.html).
- فاکتور: `/order/{id}/invoice` HTML با `@media print` (بدون dep PDF). (checkout.rs، invoice.html، print CSS).
- ردیابی سفارش: خط‌زمانی موجود + برچسب مرحله/تاریخ هر گام. (order_result.html).

## U6 — عمق صفحهٔ محصول و سبد
- پرسش‌وپاسخ: بخش + فرم در محصول؛ مدیریت/پاسخ ادمین. (0015، db/qa.rs، product.rs/html، routes/admin_qa.rs).
- نشان ارسال (زنجان/سراسر کشور)، اشتراک‌گذاری (Web Share + کپی لینک)، خرید سریع (افزودن→تسویه). (product.html، app.js).
- تایمر تخفیف محصول + قیمت قبل/درصد. (product.html).
- سبد: پیشنهاد محصولات مکمل (هم‌دسته)، بازدیدهای اخیر، نشانگر موجودی محدود. (cart.rs/html).

## U7 — عمق جستجو/کاتالوگ
- فیلترهای بیشتر: امتیاز، فقط موجود، ارسال فوری، (تخفیف‌دار). (catalog.rs، catalog.html، ProductQuery).
- مرتب‌سازی: پرفروش‌ترین، بیشترین تخفیف + موجودها. (catalog.rs).
- نمای گریدی/لیستی (toggle، کوکی)، تاریخچهٔ جستجو (localStorage)، جستجوهای محبوب. (catalog.html، app.js، search partial).

## U8 — پوشش پنل ادمین برای سیستم‌های جدید
- بنرها، تخفیف/پیشنهاد لحظه‌ای (ست deal_ends_at + compare_at)، کیف پول (شارژ/کسر دستی + گزارش)، کارت هدیه (صدور/فهرست)، Q&A (پاسخ/تأیید)، اعلان همگانی (broadcast)، settings سطوح باشگاه/ارسال/کارت‌هدیه. (routes/admin_*، admin/_nav.html، dashboard کارت‌ها).

## PROD — انتشار
- کلیدها: درگاه (zarinpal/zibal merchant + enabled)، پیامک (kavenegar/smsir + template)، SMTP، توکن ربات — همه از ENV/config.local. تست end-to-end هر کدام.
- دامنه/TLS: base_url + cookie_secret تولیدی، nginx/certbot (deploy/ موجود)، theme-color/OG مطلق.
- SEO: sitemap شامل categories/offers/deals + lastmod، canonical، JSON-LD (Product offer تخفیفی + priceValidUntil، Breadcrumb، Organization/WebSite، FAQ)، robots، عنوان/توضیح هر صفحه.
- پرفورمنس: باطل‌سازی home_cache در همهٔ CRUDهای جدید، lazy/width/height تصاویر، فشرده‌سازی، (خط لولهٔ WebP در صورت دیسک).
- امنیت: CSP، CSRF فراگیر، محدودیت نرخ، hardening کوکی/هدر، مرور دسترسی نقش‌ها.
- QA نهایی: اسموک همهٔ روت‌ها (audit/_index سبک)، تست مسیرهای پول (سفارش/لغو/مرجوع/کیف پول/امتیاز/کوپن)، صفر warning، `cargo test`.

---

## ترتیب اجرا
U1 → U2 → U3 → U4 → U5 → U6 → U7 → U8 → PROD.
سایت پس از U1+U2 قابل‌انتشار است؛ U3–U8 به‌صورت لایه‌ای بدون شکستن اضافه می‌شوند. هر فاز: migration → db → route → template → CSS/JS → باطل‌سازی کش → اسموک.
