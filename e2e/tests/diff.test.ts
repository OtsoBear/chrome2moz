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
  it("strips the `temporary` key from runtime.onInstalled:fired only (Firefox temp-install-only harness noise)", () => {
    const [n] = normalizeTrace([ev("runtime.onInstalled:fired", [{ reason: "install", temporary: true }])]);
    expect(n.args).not.toContain("temporary");
    const [chrome, firefox] = normalizeTrace([
      ev("runtime.onInstalled:fired", [{ reason: "install" }]),
      ev("runtime.onInstalled:fired", [{ reason: "install", temporary: true }]),
    ]);
    expect(chrome.args).toBe(firefox.args);
  });
  it("does NOT strip a `temporary` key on any other event (scoped to onInstalled only)", () => {
    const [n] = normalizeTrace([ev("storage.local.set", [{ temporary: true, other: 1 }])]);
    expect(n.args).toContain("temporary");
  });
  it("maps positional ids in tabs.on*:fired args (not just object-keyed ids)", () => {
    // tabs.onUpdated fires as (tabId, changeInfo, tab) — tabId is a bare positional number,
    // not nested under a key named tabId/id/windowId, so ID_KEY's key-based check alone
    // would never touch it. normalizeTrace is called separately per side in the real runner
    // (each gets its own fresh idMap), so mirror that here rather than combining into one call.
    const [chrome] = normalizeTrace([ev("tabs.onUpdated:fired", [1141107017, { status: "complete" }], "background")]);
    const [firefox] = normalizeTrace([ev("tabs.onUpdated:fired", [2, { status: "complete" }], "background")]);
    expect(chrome.args).toBe(firefox.args);
    expect(chrome.args).toContain("<id:1>");
  });
  it("does not touch positional numbers on events other than tabs.on*:fired", () => {
    const [n] = normalizeTrace([ev("someApi.call", [42])]);
    expect(n.args).toBe("[42]");
  });
  it("scrubs a bare 13-digit integer (unambiguous ms-epoch) to <time>", () => {
    const [n] = normalizeTrace([ev("x.y", [{ lastAccessed: 1785383263679 }])]);
    expect(n.args).toContain("<time>");
    expect(n.args).not.toContain("1785383263679");
  });
  it("scrubs a fractional epoch-magnitude number (10-13 integer digits) to <time>", () => {
    const [n] = normalizeTrace([ev("x.y", [{ lastAccessed: 1785383263679.716 }])]);
    expect(n.args).toContain("<time>");
    expect(n.args).not.toContain("1785383263679");
  });
  it("does NOT scrub a bare 10-digit integer (too easily confused with a real id, e.g. a Chrome tab id)", () => {
    // 1141107017 is a real Chrome tab id observed in practice, not a timestamp
    const [n] = normalizeTrace([ev("x.y", [{ someId: 1141107017 }])]);
    expect(n.args).toContain("1141107017");
    expect(n.args).not.toContain("<time>");
  });
  it("a small, clearly-not-a-timestamp number survives untouched", () => {
    const [n] = normalizeTrace([ev("x.y", [{ index: 3 }])]);
    expect(n.args).toContain("3");
    expect(n.args).not.toContain("<time>");
  });
  it("a bare 10-digit positional number on a non-tabs.on*:fired event is neither id-mapped nor epoch-scrubbed", () => {
    const [n] = normalizeTrace([ev("someApi.call", [1141107017])]);
    expect(n.args).toBe("[1141107017]");
  });
  it("does not remap sentinel ids (frameId:0, tabId:-1) -- only positive ids consume an <id:N> slot", () => {
    const [n] = normalizeTrace([ev("webNavigation.onCommitted:fired", [{ frameId: 0, tabId: -1 }])]);
    expect(n.args).toContain('"frameId":0');
    expect(n.args).toContain('"tabId":-1');
    expect(n.args).not.toContain("<id:");
    // a real id appearing alongside a sentinel should still land on <id:1>, not <id:2> --
    // the sentinel must not have consumed a slot first.
    const [withReal] = normalizeTrace([ev("x.y", [{ frameId: 0, tabId: 555 }])]);
    expect(withReal.args).toBe('[{"frameId":0,"tabId":"<id:1>"}]');
  });
  it("projects Tab-shaped objects down to url/title/status/index/active, dropping engine-specific fields", () => {
    const chromeTab = {
      id: 1, windowId: 2, index: 1, active: true, status: "loading",
      frozen: false, groupId: -1, selected: true, audible: false, height: 759,
    };
    const firefoxTab = {
      id: 1, windowId: 2, index: 1, active: true, status: "loading",
      attention: false, hidden: false, isArticle: null, sharingState: { camera: false }, height: 825,
    };
    const [chrome, firefox] = normalizeTrace([
      ev("tabs.create:resolve", [chromeTab]),
      ev("tabs.create:resolve", [firefoxTab]),
    ]);
    expect(chrome.args).toBe(firefox.args);
    expect(chrome.args).not.toContain("frozen");
    expect(chrome.args).not.toContain("attention");
    expect(chrome.args).not.toContain("height");
  });
  it("keeps url/title on a Tab-shaped object when present, and omits them when absent (permission-gated)", () => {
    const [withUrl] = normalizeTrace([ev("tabs.create:resolve", [{ id: 1, windowId: 2, index: 0, active: true, url: "https://example.com/" }])]);
    expect(withUrl.args).toContain("example.com");
    const [withoutUrl] = normalizeTrace([ev("tabs.create:resolve", [{ id: 1, windowId: 2, index: 0, active: true }])]);
    expect(withoutUrl.args).not.toContain("url");
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
  it("supports an api#substring allowlist form that also requires the substring in args", () => {
    const a = normalizeTrace([ev("net.fetch", ["GET", "https://onenote.com/strings?x=1"])]);
    const allowed = diffTraces(a, [], ["net.fetch#onenote.com/strings"]);
    expect(allowed[0].allowed).toBe(true);

    const b = normalizeTrace([ev("net.fetch", ["GET", "https://example.com/other"])]);
    const notAllowed = diffTraces(b, [], ["net.fetch#onenote.com/strings"]);
    expect(notAllowed[0].allowed).toBe(false);
  });
  it("api#substring still requires the api glob to match, independent of the substring", () => {
    const a = normalizeTrace([ev("runtime.sendMessage", ["hello"])]);
    const d = diffTraces(a, [], ["net.fetch#hello"]);
    expect(d[0].allowed).toBe(false);
  });
  it("plain (no #) patterns keep matching on api name only, as before", () => {
    const a = normalizeTrace([ev("tabGroups.query", [{ anything: "at all" }])]);
    const d = diffTraces(a, [], ["tabGroups.*"]);
    expect(d[0].allowed).toBe(true);
  });
});
