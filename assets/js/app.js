// زاگرس — اسکریپت عمومی سبک (بدون وابستگی). HTMX/Alpine جدا بارگذاری می‌شوند.
(function () {
  "use strict";

  // ---- اسلایدر هیرو (Alpine) — RTL، خودکار، با کنترل دستی ----
  // ثبت روی هوک alpine:init تا فارغ از ترتیب بارگذاری اسکریپت‌ها همیشه پیش از
  // ارزیابی x-data در دسترس باشد (وگرنه کامپوننت بی‌صدا شکست می‌خورد و اسلایدر یخ می‌زند).
  document.addEventListener("alpine:init", function () {
    window.Alpine.data("heroSlider", window.heroSlider);
  });
  window.heroSlider = function (count) {
    return {
      i: 0,
      count: count,
      timer: null,
      init: function () {
        if (this.count > 1) this.start();
      },
      start: function () {
        var self = this;
        this.timer = setInterval(function () { self.next(); }, 6000);
      },
      reset: function () {
        clearInterval(this.timer);
        if (this.count > 1) this.start();
      },
      apply: function () {
        // در RTL، اسلاید با ترنسلیت مثبت جابه‌جا می‌شود.
        if (this.$refs.track)
          this.$refs.track.style.transform = "translateX(" + (this.i * 100) + "%)";
      },
      go: function (n) { this.i = n; this.apply(); this.reset(); },
      next: function () { this.i = (this.i + 1) % this.count; this.apply(); },
      prev: function () { this.i = (this.i - 1 + this.count) % this.count; this.apply(); this.reset(); },
    };
  };

  // ---- انتخاب تنوع محصول: قیمت زنده + هم‌گام‌سازی variant_id ----
  function initVariantPicker() {
    var sel = document.getElementById("variantSelect");
    if (!sel) return;
    var hidden = document.getElementById("variantIdInput");
    var priceEl = document.getElementById("variantPrice");
    var addBtn = document.getElementById("addToCartBtn");
    var qty = document.getElementById("qtyInput");
    var compareEl = document.getElementById("variantCompare");
    var pctEl = document.getElementById("variantPct");

    function apply() {
      var opt = sel.options[sel.selectedIndex];
      if (!opt) return;
      if (hidden) hidden.value = opt.value;
      var price = opt.getAttribute("data-price-fa");
      if (price && priceEl) priceEl.textContent = price;
      // قیمت قبل + درصد تخفیف این تنوع (در صورت وجود).
      var cmp = opt.getAttribute("data-compare-fa");
      var pct = opt.getAttribute("data-pct");
      if (compareEl) {
        if (cmp) { compareEl.textContent = cmp; compareEl.hidden = false; }
        else compareEl.hidden = true;
      }
      if (pctEl) {
        if (pct) { pctEl.textContent = pct + "٪"; pctEl.hidden = false; }
        else pctEl.hidden = true;
      }
      var stock = parseInt(opt.getAttribute("data-stock") || "0", 10);
      if (qty) qty.max = stock > 0 ? stock : 1;
      if (qty && parseInt(qty.value, 10) > stock) qty.value = stock > 0 ? stock : 1;
      if (addBtn) {
        addBtn.disabled = stock <= 0;
        addBtn.textContent = stock <= 0 ? "ناموجود" : "افزودن به سبد خرید";
      }
    }
    sel.addEventListener("change", apply);
    apply();
  }

  // ---- کنترل تعداد (+/-) با سقف موجودی ----
  function initQtyControls() {
    document.querySelectorAll("[data-qty-control]").forEach(function (ctrl) {
      var input = ctrl.querySelector("input");
      if (!input) return;
      ctrl.querySelectorAll("[data-qty-step]").forEach(function (btn) {
        btn.addEventListener("click", function () {
          var step = parseInt(btn.getAttribute("data-qty-step"), 10) || 0;
          var max = parseInt(input.max || "0", 10);
          var min = parseInt(input.min || "1", 10);
          var v = (parseInt(input.value, 10) || min) + step;
          if (v < min) v = min;
          if (max > 0 && v > max) v = max;
          input.value = v;
          input.dispatchEvent(new Event("change", { bubbles: true }));
        });
      });
    });
  }

  // ---- گالری تصویر محصول + لایت‌باکس ----
  function initGallery() {
    var main = document.getElementById("mainImage");
    if (!main) return;
    document.querySelectorAll("[data-gallery-thumb]").forEach(function (t) {
      t.addEventListener("click", function () {
        var src = t.getAttribute("data-src");
        if (src) main.src = src;
        document.querySelectorAll("[data-gallery-thumb]").forEach(function (x) {
          x.classList.remove("active");
        });
        t.classList.add("active");
      });
    });
    var box = document.getElementById("lightbox");
    if (box) {
      var img = box.querySelector("img");
      main.addEventListener("click", function () {
        img.src = main.src;
        box.classList.add("open");
      });
      box.addEventListener("click", function () {
        box.classList.remove("open");
      });
    }
  }

  // ---- شمارش معکوس پیشنهاد لحظه‌ای ----
  function initDealCountdowns() {
    var els = document.querySelectorAll("[data-deal-countdown]");
    if (!els.length) return;
    var fa = function (n) {
      return String(n).replace(/\d/g, function (d) { return "۰۱۲۳۴۵۶۷۸۹"[d]; });
    };
    var pad = function (n) { return n < 10 ? "0" + n : "" + n; };
    function tick() {
      var now = Date.now();
      els.forEach(function (el) {
        var clock = el.querySelector("[data-deal-clock]");
        if (!clock) return;
        var end = new Date(el.getAttribute("data-deal-countdown")).getTime();
        var diff = Math.floor((end - now) / 1000);
        if (isNaN(end) || diff <= 0) {
          el.classList.add("is-ended");
          clock.textContent = "پایان یافت";
          return;
        }
        var d = Math.floor(diff / 86400);
        var h = Math.floor((diff % 86400) / 3600);
        var m = Math.floor((diff % 3600) / 60);
        var s = diff % 60;
        var txt = (d > 0 ? fa(d) + " روز " : "") + fa(pad(h)) + ":" + fa(pad(m)) + ":" + fa(pad(s));
        clock.textContent = txt;
      });
    }
    tick();
    setInterval(tick, 1000);
  }

  // ---- اشتراک‌گذاری (Web Share → کلیپ‌بورد → fallback قدیمی)، همیشه با بازخورد ----
  function legacyCopy(text) {
    try {
      var ta = document.createElement("textarea");
      ta.value = text; ta.setAttribute("readonly", "");
      ta.style.position = "fixed"; ta.style.opacity = "0";
      document.body.appendChild(ta); ta.select();
      var ok = document.execCommand("copy");
      document.body.removeChild(ta);
      return ok;
    } catch (e) { return false; }
  }
  function initShare() {
    document.querySelectorAll(".share-btn").forEach(function (btn) {
      if (btn._shareBound) return;
      btn._shareBound = true;
      btn.addEventListener("click", function (e) {
        e.preventDefault();
        var url = btn.getAttribute("data-share-url") || location.href;
        var title = btn.getAttribute("data-share-title") || document.title;
        if (navigator.share) {
          navigator.share({ title: title, url: url }).catch(function () {});
          return;
        }
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(url).then(
            function () { toast("لینک کپی شد ✓"); },
            function () { toast(legacyCopy(url) ? "لینک کپی شد ✓" : "کپی نشد — لینک: " + url); }
          );
          return;
        }
        toast(legacyCopy(url) ? "لینک کپی شد ✓" : "لینک: " + url);
      });
    });
  }

  // ---- تغییر نمای گرید/لیست (با ذخیره در localStorage) ----
  function initViewToggle() {
    var grid = document.querySelector(".product-grid[data-view]");
    var toggle = document.querySelector("[data-view-toggle]");
    if (!grid || !toggle) return;
    function apply(mode) {
      grid.classList.toggle("list-view", mode === "list");
      toggle.querySelectorAll("[data-view-mode]").forEach(function (b) {
        b.classList.toggle("is-active", b.getAttribute("data-view-mode") === mode);
      });
    }
    var saved = localStorage.getItem("zagros_view") || "grid";
    apply(saved);
    toggle.querySelectorAll("[data-view-mode]").forEach(function (b) {
      b.addEventListener("click", function () {
        var mode = b.getAttribute("data-view-mode");
        localStorage.setItem("zagros_view", mode);
        apply(mode);
      });
    });
  }

  // ---- تاریخچه‌ی جستجو (localStorage) + جستجوهای محبوب ----
  var POPULAR = ["یخچال", "فریزر", "آبسردکن", "جاروبرقی"];
  function recordSearch() {
    var m = location.search.match(/[?&]q=([^&]+)/);
    if (!m) return;
    var q = decodeURIComponent(m[1].replace(/\+/g, " ")).trim();
    if (!q) return;
    var hist = JSON.parse(localStorage.getItem("zagros_searches") || "[]");
    hist = hist.filter(function (x) { return x !== q; });
    hist.unshift(q);
    localStorage.setItem("zagros_searches", JSON.stringify(hist.slice(0, 6)));
  }
  function initSearchHistory() {
    var input = document.querySelector(".header-search-input");
    var box = document.getElementById("searchSuggest");
    if (!input || !box) return;
    input.addEventListener("focus", function () {
      if (input.value.trim() !== "") return;
      var hist = JSON.parse(localStorage.getItem("zagros_searches") || "[]");
      var rows = "";
      if (hist.length) {
        rows += '<div class="search-suggest-head">جستجوهای اخیر</div>';
        hist.forEach(function (q) {
          rows += '<a href="/products?q=' + encodeURIComponent(q) + '" class="search-suggest-row">' + q + "</a>";
        });
      }
      rows += '<div class="search-suggest-head">محبوب</div>';
      POPULAR.forEach(function (q) {
        rows += '<a href="/products?q=' + encodeURIComponent(q) + '" class="search-suggest-row">' + q + "</a>";
      });
      box.innerHTML = '<div class="search-suggest-list">' + rows + "</div>";
    });
  }

  // ---- بازدیدهای اخیر (localStorage) ----
  var RV_KEY = "zagros_rv";
  function rvEsc(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c];
    });
  }
  function rvLoad() {
    try { return JSON.parse(localStorage.getItem(RV_KEY) || "[]"); } catch (e) { return []; }
  }
  function recordRecentlyViewed() {
    var el = document.getElementById("rvProduct");
    if (!el) return;
    var slug = el.getAttribute("data-slug");
    if (!slug) return;
    var item = {
      slug: slug,
      title: el.getAttribute("data-title") || "",
      img: el.getAttribute("data-img") || "",
      price: el.getAttribute("data-price") || "",
    };
    var list = rvLoad().filter(function (x) { return x && x.slug !== slug; });
    list.unshift(item);
    try { localStorage.setItem(RV_KEY, JSON.stringify(list.slice(0, 12))); } catch (e) {}
  }
  function renderRecentlyViewed() {
    var sec = document.getElementById("recentlyViewed");
    if (!sec) return;
    var grid = sec.querySelector(".product-grid");
    var list = rvLoad();
    if (!grid || !list.length) return;
    var html = "";
    list.slice(0, 6).forEach(function (it) {
      if (!it || !it.slug) return;
      var thumb = it.img
        ? '<img src="' + rvEsc(it.img) + '" alt="' + rvEsc(it.title) + '" loading="lazy" width="300" height="300">'
        : '<div class="product-thumb-placeholder" aria-hidden="true">زاگرس</div>';
      var price = it.price
        ? '<div class="product-price-row"><span class="product-price">' + rvEsc(it.price) + "</span></div>"
        : "";
      html +=
        '<article class="product-card"><a href="/product/' + encodeURIComponent(it.slug) + '" class="product-card-link">' +
        '<div class="product-thumb">' + thumb + "</div>" +
        '<div class="product-card-body"><h3 class="product-name">' + rvEsc(it.title) + "</h3>" + price + "</div>" +
        "</a></article>";
    });
    if (!html) return;
    grid.innerHTML = html;
    sec.hidden = false;
    var clr = document.getElementById("rvClear");
    if (clr) {
      clr.addEventListener("click", function () {
        try { localStorage.removeItem(RV_KEY); } catch (e) {}
        sec.hidden = true;
      });
    }
  }

  function ready(fn) {
    if (document.readyState !== "loading") fn();
    else document.addEventListener("DOMContentLoaded", fn);
  }
  ready(function () {
    initVariantPicker();
    initQtyControls();
    initGallery();
    initDealCountdowns();
    initShare();
    initViewToggle();
    recordSearch();
    initSearchHistory();
    recordRecentlyViewed();
    renderRecentlyViewed();
  });

  // برای محتوای تزریق‌شده با HTMX دوباره مقداردهی می‌کنیم.
  document.body.addEventListener("htmx:afterSwap", function () {
    initQtyControls();
  });

  // ---- toast ساده ----
  function toast(msg) {
    var t = document.getElementById("zagros-toast");
    if (!t) {
      t = document.createElement("div");
      t.id = "zagros-toast";
      t.className = "toast";
      document.body.appendChild(t);
    }
    t.textContent = msg;
    t.classList.add("show");
    clearTimeout(t._timer);
    t._timer = setTimeout(function () {
      t.classList.remove("show");
    }, 2500);
  }
  // وقتی سبد تغییر کرد (افزودن با HTMX)، toast نشان بده.
  document.body.addEventListener("cart-changed", function () {
    toast("به سبد خرید اضافه شد ✓");
  });
})();
