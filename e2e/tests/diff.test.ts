import { describe, it, expect } from "vitest";
import { normalizeTrace, diffTraces } from "../src/diff.js";

const ev = (api: string, args: unknown[] = [], ctx = "background") =>
  ({ seq: 0, ctx, api, args });

describe("normalizeTrace", () => {
  it("maps ids to stable placeholders by first appearance", () => {
    const [a, b] = normalizeTrace([ev("tabs.sendMessage", [{ tabId: 731 }]), ev("tabs.sendMessage", [{ tabId: 731 }])]);
    expect(a.args).toContain("<id:1>");
    expect(a.args).toBe(b.args);
  });
  it("scrubs extension-origin urls and timestamps", () => {
    const [n] = normalizeTrace([ev("net.fetch", ["GET", "chrome-extension://abcdefgh/popup.html?t=1753791234567"])]);
    expect(n.args).not.toContain("abcdefgh");
    expect(n.args).not.toContain("1753791234567");
  });
});

describe("diffTraces", () => {
  it("returns empty for identical traces", () => {
    const a = normalizeTrace([ev("storage.local.set", [{ a: 1 }])]);
    expect(diffTraces(a, a, [])).toEqual([]);
  });
  it("flags one-sided events with the side that has them", () => {
    const a = normalizeTrace([ev("storage.local.set", [{}]), ev("management.uninstallSelf")]);
    const b = normalizeTrace([ev("storage.local.set", [{}])]);
    const d = diffTraces(a, b, []);
    expect(d).toHaveLength(1);
    expect(d[0].side).toBe("a");
    expect(d[0].event.api).toBe("management.uninstallSelf");
    expect(d[0].allowed).toBe(false);
  });
  it("marks divergences matching allowed_diffs globs", () => {
    const a = normalizeTrace([ev("tabGroups.query", [{}])]);
    const d = diffTraces(a, [], ["tabGroups.*"]);
    expect(d[0].allowed).toBe(true);
  });
  it("detects content divergence when same api has different args", () => {
    const a = normalizeTrace([ev("storage.local.set", [{ a: 1 }])]);
    const b = normalizeTrace([ev("storage.local.set", [{ a: 2 }])]);
    const d = diffTraces(a, b, []);
    expect(d).toHaveLength(2);
    expect(d.some((x) => x.side === "a")).toBe(true);
    expect(d.some((x) => x.side === "b")).toBe(true);
    expect(d.every((x) => x.allowed === false)).toBe(true);
  });
});
