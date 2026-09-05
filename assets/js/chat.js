// کلاینت چت زنده‌ی زاگرس — سبک، بدون وابستگی، خودمیزبان.
// از کانتینر #chat مقادیر data-role و data-ws-path را می‌خواند.
(function () {
  "use strict";
  var root = document.getElementById("chat");
  if (!root) return;

  var log = document.getElementById("chat-log");
  var form = document.getElementById("chat-form");
  var input = document.getElementById("chat-input");
  var statusEl = document.getElementById("chat-status");

  var role = root.dataset.role || "customer";
  var wsPath = root.dataset.wsPath;
  var proto = location.protocol === "https:" ? "wss:" : "ws:";
  var wsUrl = proto + "//" + location.host + wsPath;

  var ws = null;
  var reconnectTimer = null;

  function setStatus(text, ok) {
    if (!statusEl) return;
    statusEl.textContent = text;
    statusEl.classList.toggle("is-online", !!ok);
  }

  function formatTime(iso) {
    try {
      return new Date(iso).toLocaleTimeString("fa-IR", {
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch (e) {
      return "";
    }
  }

  // ساخت حباب پیام با textContent (ایمن در برابر XSS).
  function addMessage(m) {
    var isStaff = m.sender === "staff";
    var mine = m.sender === role;

    var wrap = document.createElement("div");
    wrap.className =
      "chat-msg " + (isStaff ? "is-staff" : "is-customer") + (mine ? " is-mine" : "");

    var bubble = document.createElement("div");
    bubble.className = "chat-bubble";
    bubble.textContent = m.body;

    var meta = document.createElement("div");
    meta.className = "chat-meta";
    meta.textContent = (isStaff ? "پشتیبانی" : "مشتری") + " • " + formatTime(m.at);

    wrap.appendChild(bubble);
    wrap.appendChild(meta);
    log.appendChild(wrap);
    log.scrollTop = log.scrollHeight;
  }

  function connect() {
    setStatus("در حال اتصال…", false);
    ws = new WebSocket(wsUrl);

    ws.onopen = function () {
      setStatus("متصل", true);
    };
    ws.onmessage = function (e) {
      try {
        addMessage(JSON.parse(e.data));
      } catch (err) {
        /* پیام نامعتبر را نادیده می‌گیریم */
      }
    };
    ws.onclose = function () {
      setStatus("قطع شد — تلاش مجدد…", false);
      clearTimeout(reconnectTimer);
      reconnectTimer = setTimeout(connect, 2000);
    };
    ws.onerror = function () {
      if (ws) ws.close();
    };
  }

  form.addEventListener("submit", function (e) {
    e.preventDefault();
    var text = input.value.trim();
    if (!text || !ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(text);
    input.value = "";
    input.focus();
  });

  connect();
})();
