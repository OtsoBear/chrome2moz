import { describe, it, expect, beforeEach, vi } from "vitest";
import { readFileSync } from "node:fs";
import vm from "node:vm";

function loadShim(fakeChrome: any) {
  const src = readFileSync(new URL("../shim/shim.js", import.meta.url), "utf8")
    .replaceAll("__C2M_SIDE__", "chrome-orig")
    .replaceAll("__C2M_PORT__", "41999");
  const posted: any[] = [];
  const sandbox: any = {
    chrome: fakeChrome,
    fetch: vi.fn(async (url: string, init?: any) => {
      if (init?.body) posted.push(JSON.parse(init.body));
      return { ok: true, status: 204, json: async () => ({}) };
    }),
    setInterval: (fn: () => void) => fn, // capture, don't schedule
    console,
  };
  sandbox.globalThis = sandbox;
  sandbox.self = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(src, sandbox);
  return { sandbox, posted, flush: () => sandbox.__c2m_test_flush__() };
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
});
