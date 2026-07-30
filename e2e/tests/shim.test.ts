import { describe, it, expect, beforeEach, vi } from "vitest";
import { readFileSync } from "node:fs";
import vm from "node:vm";

function loadShim(fakeChrome: any, opts: { cmdQueue?: any[] } = {}) {
  const src = readFileSync(new URL("../shim/shim.js", import.meta.url), "utf8")
    .replaceAll("__C2M_SIDE__", "chrome-orig")
    .replaceAll("__C2M_PORT__", "41999");
  const posted: any[] = []; // bodies POSTed to /trace only
  const cmdResults: any[] = []; // bodies POSTed to /cmdresult only
  const intervals: Array<() => void> = [];
  const cmdQueue = opts.cmdQueue ? [...opts.cmdQueue] : [];
  const sandbox: any = {
    chrome: fakeChrome,
    fetch: vi.fn(async (url: string, init?: any) => {
      const path = typeof url === "string" ? url.replace("http://127.0.0.1:41999", "") : "";
      if (init?.body && path.startsWith("/trace")) posted.push(JSON.parse(init.body));
      else if (init?.body && path.startsWith("/cmdresult")) cmdResults.push(JSON.parse(init.body));
      if (path.startsWith("/cmd") && !path.startsWith("/cmdresult")) {
        const cmd = cmdQueue.shift();
        if (!cmd) return { ok: true, status: 204, json: async () => ({}) };
        return { ok: true, status: 200, json: async () => cmd };
      }
      return { ok: true, status: 204, json: async () => ({}) };
    }),
    // capture, don't schedule — tests invoke captured callbacks (e.g. the poll loop) manually
    setInterval: (fn: () => void) => { intervals.push(fn); return fn; },
    console,
  };
  sandbox.globalThis = sandbox;
  sandbox.self = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(src, sandbox);
  return { sandbox, posted, cmdResults, intervals, flush: () => sandbox.__c2m_test_flush__() };
}

describe("shim", () => {
  let calls: any[];
  let fake: any;
  beforeEach(() => {
    calls = [];
    fake = {
      storage: { local: { set: (v: any) => { calls.push(["set", v]); return Promise.resolve(); } } },
      runtime: {
        sendMessage: (m: any) => { calls.push(["send", m]); return Promise.resolve(); },
        onMessage: { addListener: (cb: any) => calls.push(["listen", cb]) },
      },
    };
  });

  it("records wrapped API calls and still calls through", async () => {
    const { flush, posted, sandbox } = loadShim(fake);
    await sandbox.chrome.storage.local.set({ a: 1 });
    flush();
    const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
    expect(apis).toContain("storage.local.set");
    expect(calls).toContainEqual(["set", { a: 1 }]);
  });

  it("does not record shim-internal messages", async () => {
    const { flush, posted, sandbox } = loadShim(fake);
    await sandbox.chrome.runtime.sendMessage({ __c2m__: "ping" });
    flush();
    const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
    expect(apis).not.toContain("runtime.sendMessage");
  });

  it("never adds missing namespaces (transparency)", () => {
    const { sandbox } = loadShim(fake);
    expect(sandbox.chrome.offscreen).toBeUndefined();
    expect(sandbox.chrome.tabGroups).toBeUndefined();
  });

  it("does not record its own ping-relay tabs.query/tabs.sendMessage traffic", async () => {
    fake.tabs = {
      query: (q: any) => { calls.push(["query", q]); return Promise.resolve([{ id: 7 }]); },
      sendMessage: (id: number, m: any) => { calls.push(["sendMessage", id, m]); return Promise.resolve({ __c2m__: "pong" }); },
    };
    const { flush, posted, intervals } = loadShim(fake, { cmdQueue: [{ type: "ping" }] });
    // the background-context poll loop is the last setInterval registered (after the flush interval)
    const poll = intervals[intervals.length - 1];
    await poll();
    flush();
    const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
    expect(apis.some((a: string) => a.startsWith("tabs."))).toBe(false);
    // the real tabs.query/tabs.sendMessage calls still happened (transparency: call-through preserved)
    expect(calls).toContainEqual(["query", { active: true }]);
    expect(calls).toContainEqual(["sendMessage", 7, { __c2m__: "ping" }]);
  });

  it("wrapEvent preserves removeListener with the caller's original callback", () => {
    const listeners = new Set<any>();
    fake.runtime.onMessage = {
      addListener: (cb: any) => { listeners.add(cb); },
      removeListener: (cb: any) => { listeners.delete(cb); },
    };
    const { sandbox } = loadShim(fake);
    const before = new Set(listeners);

    const origCb = () => {};
    sandbox.chrome.runtime.onMessage.addListener(origCb);
    const added = [...listeners].filter((l) => !before.has(l));
    expect(added).toHaveLength(1);
    expect(added[0]).not.toBe(origCb); // the store holds the wrapped listener, not the raw cb

    sandbox.chrome.runtime.onMessage.removeListener(origCb);
    expect(listeners).toEqual(before); // removeListener(origCb) found and removed the wrapped one
  });
});
