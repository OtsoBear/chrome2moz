/* chrome2moz e2e spy shim. Injected first into every extension context.
   Placeholders __C2M_SIDE__ / __C2M_PORT__ / __C2M_CTX_OVERRIDE__ are replaced at injection time. */
(() => {
  const g = globalThis;
  if (g.__c2m_shim__) return;
  g.__c2m_shim__ = true;

  const SIDE = "__C2M_SIDE__";
  const PORT = __C2M_PORT__;
  const BASE = "http://127.0.0.1:" + PORT;
  const MARK = "__c2m__";
  // Baked in by the injector for the background entry point only. Chrome's MV3 service
  // worker and Firefox's converted event-page background run under different file names
  // (__c2m_bg.js vs _generated_background_page.html) even though they're the same logical
  // context, so location-based ctx detection can't tell them apart across sides. The
  // override collapses both onto one canonical "background" ctx label.
  const CTX_OVERRIDE = "__C2M_CTX_OVERRIDE__";

  const ctx = CTX_OVERRIDE || (() => {
    try {
      if (typeof location === "undefined") return "background";
      const p = location.protocol;
      if (p === "chrome-extension:" || p === "moz-extension:") return "extpage:" + location.pathname;
      if (p === "http:" || p === "https:") return "content";
      return "background";
    } catch { return "background"; }
  })();

  let seq = 0;
  const buf = [];
  const record = (api, args) => { buf.push({ seq: seq++, ctx, api, args }); };

  const MAXSTR = 200;
  const norm = (v, depth = 0) => {
    if (depth > 4) return "[deep]";
    if (v === undefined) return null;
    if (typeof v === "function") return "[fn]";
    if (typeof v === "string") return v.length > MAXSTR ? v.slice(0, MAXSTR) + "…" : v;
    if (v === null || typeof v !== "object") return v;
    if (Array.isArray(v)) return v.slice(0, 20).map((x) => norm(x, depth + 1));
    const o = {};
    for (const k of Object.keys(v).slice(0, 30)) {
      try { o[k] = norm(v[k], depth + 1); } catch { o[k] = "[err]"; }
    }
    return o;
  };
  const normArgs = (args) => Array.from(args, (a) => norm(a));

  const rawFetch = g.fetch ? g.fetch.bind(g) : null;
  const flush = () => {
    if (!buf.length || !rawFetch) return;
    const events = buf.splice(0);
    try {
      rawFetch(BASE + "/trace", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ side: SIDE, events }),
      }).catch(() => {});
    } catch {}
  };
  g.__c2m_test_flush__ = flush;
  const timer = typeof setInterval === "function" ? setInterval(flush, 250) : null;

  const isInternal = (args) =>
    args.length > 0 && args[0] && typeof args[0] === "object" && MARK in args[0];

  const wrapFn = (ns, path, key) => {
    const orig = ns[key];
    const api = path + "." + key;
    const wrapped = function (...args) {
      try { if (!isInternal(args)) record(api, normArgs(args)); } catch {}
      let r;
      try { r = orig.apply(ns, args); }
      catch (e) { try { record(api + ":throw", [String(e)]); } catch {} throw e; }
      if (r && typeof r.then === "function") {
        r.then(
          (v) => { try { record(api + ":resolve", [norm(v)]); } catch {} },
          (e) => { try { record(api + ":reject", [String(e)]); } catch {} },
        );
      }
      return r;
    };
    try { ns[key] = wrapped; } catch {}
  };

  const wrapEvent = (ev, path) => {
    const origAdd = ev.addListener;
    if (typeof origAdd !== "function") return;
    // Maps the caller's original callback to the wrapped one we actually register,
    // so removeListener/hasListener (called with the original cb) keep working.
    const cbMap = new WeakMap();
    try {
      ev.addListener = function (cb, ...rest) {
        const wrappedCb = function (...args) {
          try { if (!isInternal(args)) record(path + ":fired", normArgs(args)); } catch {}
          return cb.apply(this, args);
        };
        try { cbMap.set(cb, wrappedCb); } catch {}
        return origAdd.call(ev, wrappedCb, ...rest);
      };
    } catch {}

    const origRemove = ev.removeListener;
    if (typeof origRemove === "function") {
      try {
        ev.removeListener = function (cb, ...rest) {
          let w;
          try { w = cbMap.get(cb); } catch {}
          return origRemove.call(ev, w || cb, ...rest);
        };
      } catch {}
    }

    const origHas = ev.hasListener;
    if (typeof origHas === "function") {
      try {
        ev.hasListener = function (cb, ...rest) {
          let w;
          try { w = cbMap.get(cb); } catch {}
          return origHas.call(ev, w || cb, ...rest);
        };
      } catch {}
    }
  };

  const SKIP = new Set(["csi", "loadTimes"]);
  const walk = (ns, path, depth) => {
    if (!ns || typeof ns !== "object" || depth > 3) return;
    for (const key of Object.keys(ns)) {
      if (SKIP.has(key)) continue;
      let v;
      try { v = ns[key]; } catch { continue; }
      const p = path + "." + key;
      if (typeof v === "function") wrapFn(ns, path, key);
      else if (v && typeof v === "object") {
        if (typeof v.addListener === "function") wrapEvent(v, p);
        else walk(v, p, depth + 1);
      }
    }
  };

  const root = typeof browser !== "undefined" ? browser : typeof chrome !== "undefined" ? chrome : null;

  // Capture raw tabs.query/tabs.sendMessage BEFORE the walk below wraps them, so the
  // shim's own ping-relay polling (below) calls the unwrapped functions and never
  // shows up as tabs.* trace events — mirrors the rawFetch pattern.
  const rawTabsQuery = root && root.tabs && typeof root.tabs.query === "function"
    ? root.tabs.query.bind(root.tabs) : null;
  const rawTabsSendMessage = root && root.tabs && typeof root.tabs.sendMessage === "function"
    ? root.tabs.sendMessage.bind(root.tabs) : null;

  if (root) {
    for (const top of Object.keys(root)) {
      let v;
      try { v = root[top]; } catch { continue; }
      if (v && typeof v === "object") {
        if (typeof v.addListener === "function") wrapEvent(v, top);
        else walk(v, top, 0);
      } else if (typeof v === "function") wrapFn(root, "", top);
    }
    // In Chrome, `browser` may be an alias of `chrome`; wrapping once via the root reference covers both.
  }

  if (rawFetch) {
    g.fetch = function (input, init) {
      try {
        const url = typeof input === "string" ? input : input && input.url ? input.url : String(input);
        if (!url.startsWith(BASE)) record("net.fetch", [init && init.method ? init.method : "GET", norm(url)]);
      } catch {}
      return rawFetch(input, init);
    };
  }

  try {
    g.addEventListener?.("error", (e) => { try { record("runtime.error", [norm(String(e && e.message))]); } catch {} });
    g.addEventListener?.("unhandledrejection", (e) => { try { record("runtime.error", [norm(String(e && e.reason))]); } catch {} });
  } catch {}

  // Command channel + ping relay (background only)
  if (ctx === "background" && root && rawFetch) {
    if (root.runtime && root.runtime.onMessage) {
      try {
        root.runtime.onMessage.addListener((msg, _s, sendResponse) => {
          if (msg && typeof msg === "object" && msg[MARK] === "ping") { sendResponse({ [MARK]: "pong" }); return true; }
        });
      } catch {}
    }
    const poll = async () => {
      try {
        const res = await rawFetch(BASE + "/cmd?side=" + SIDE);
        if (res.status !== 200) return;
        const cmd = await res.json();
        if (cmd.type === "ping") {
          let ok = false;
          try {
            const tabs = rawTabsQuery ? await rawTabsQuery({ active: true }) : [];
            const reply = rawTabsSendMessage ? await rawTabsSendMessage(tabs[0].id, { [MARK]: "ping" }) : null;
            ok = !!(reply && reply[MARK] === "pong");
          } catch {}
          await rawFetch(BASE + "/cmdresult", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ side: SIDE, result: { type: "ping", ok } }),
          });
        }
      } catch {}
    };
    if (typeof setInterval === "function") setInterval(poll, 500);
  } else if (ctx === "content" && root && root.runtime && root.runtime.onMessage) {
    try {
      root.runtime.onMessage.addListener((msg, _s, sendResponse) => {
        if (msg && typeof msg === "object" && msg[MARK] === "ping") { sendResponse({ [MARK]: "pong" }); return true; }
      });
    } catch {}
  }

  void timer;
})();
