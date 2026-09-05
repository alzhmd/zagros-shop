#!/usr/bin/env python3
"""اسکرین‌شات‌گیری بصری از زاگرس (دسکتاپ + موبایل) با Playwright.
کوکی نشست ادمین از فایل Netscape cookie جار curl خوانده می‌شود تا صفحات
احراز‌هویتی هم گرفته شوند."""
import sys, os, re
from playwright.sync_api import sync_playwright

BASE = "http://127.0.0.1:8080"
OUT = "/tmp/shots"
os.makedirs(OUT, exist_ok=True)

def load_cookies(path):
    cookies = []
    if not os.path.exists(path):
        return cookies
    for line in open(path):
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            # خط HttpOnly در فرمت curl با #HttpOnly_ شروع می‌شود
            if line.startswith("#HttpOnly_"):
                line = line[len("#HttpOnly_"):]
            else:
                continue
        parts = line.split("\t")
        if len(parts) != 7:
            continue
        domain, flag, path_, secure, expiry, name, value = parts
        cookies.append({
            "name": name, "value": value,
            "domain": "127.0.0.1", "path": path_ or "/",
        })
    return cookies

# pages: (slug, url, modes)  modes: "d"=desktop "m"=mobile
PAGES = [
    ("home",        "/",                                "dm"),
    ("catalog",     "/products",                        "dm"),
    ("catalog-list","/products?off=1",                  "d"),
    ("product",     "/product/parskhazar-vc-2000",      "dm"),
    ("categories",  "/categories",                      "dm"),
    ("offers",      "/offers",                          "d"),
    ("cart",        "/cart",                            "dm"),
    ("checkout",    "/checkout",                        "d"),
    ("login",       "/account/login",                   "m"),
    ("account",     "/account",                         "dm"),
    ("club",        "/account/club",                    "d"),
    ("wallet",      "/account/wallet",                  "d"),
    ("notifs",      "/account/notifications",           "d"),
    ("admin",       "/admin",                           "d"),
    ("admin-prod",  "/admin/products/55555555-0000-0000-0000-000000000005/edit", "d"),
    ("admin-set",   "/admin/settings",                  "d"),
    ("admin-banner","/admin/banners",                   "d"),
]

def main():
    cookies = load_cookies("/tmp/admincj.txt")
    print(f"loaded {len(cookies)} cookies")
    shots = []
    with sync_playwright() as p:
        browser = p.chromium.launch(args=["--no-sandbox"])
        for mode, vp, mobile in [("d", {"width":1280,"height":900}, False),
                                  ("m", {"width":390,"height":844}, True)]:
            ctx = browser.new_context(viewport=vp, device_scale_factor=1,
                                      is_mobile=mobile, locale="fa-IR")
            if cookies:
                ctx.add_cookies(cookies)
            page = ctx.new_page()
            for slug, url, modes in PAGES:
                if mode not in modes:
                    continue
                try:
                    page.goto(BASE + url, wait_until="networkidle", timeout=15000)
                    page.wait_for_timeout(700)  # اجازه به Alpine/HTMX
                    fn = f"{OUT}/{slug}-{mode}.png"
                    page.screenshot(path=fn, full_page=True)
                    shots.append(fn)
                    print("OK", fn)
                except Exception as e:
                    print("ERR", slug, mode, repr(e)[:120])
            ctx.close()
        browser.close()
    print("TOTAL", len(shots))

if __name__ == "__main__":
    main()
