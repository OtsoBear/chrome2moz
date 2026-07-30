import picomatch from "picomatch";
import type { TraceEvent } from "./telemetry.js";

export type NormalizedEvent = { ctx: string; api: string; args: string };
export type Divergence = { side: "a" | "b"; event: NormalizedEvent; allowed: boolean };

const ID_KEY = /^(tabId|windowId|frameId|requestId|id)$/;
const EXT_URL = /(chrome|moz)-extension:\/\/[a-z0-9-]+/gi;
const EPOCH = /\b1[0-9]{9}(?:[0-9]{3})?\b/g;
const ISO = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[^"\s]*/g;
// tabs.on*:fired listener signatures carry ids positionally, not under an ID_KEY-named
// object key: onUpdated(tabId, changeInfo, tab), onRemoved(tabId, removeInfo),
// onCreated(tab) [no positional id]. Only the array elements of these specific events get
// checked for bare numeric ids; nothing else does (a bare number elsewhere is not assumed
// to be an id).
const TABS_FIRED = /^tabs\.on\w+:fired$/;
// browser.tabs.create/query/onCreated/onUpdated all hand back the browser's native tabs.Tab
// object, whose shape genuinely (and irrelevantly, for our purposes) differs between Chrome
// and Firefox — e.g. Chrome-only frozen/groupId/selected vs Firefox-only
// attention/hidden/isArticle/isInReaderMode/sharingState/successorTabId/cookieStoreId — and
// even shared fields like lastAccessed/width/height carry engine-specific noise (differing
// chrome UI dimensions, sub-millisecond timing). Rather than allowlisting every API that can
// hand back a Tab, project any Tab-shaped object down to the handful of fields an extension
// actually cares about. id/windowId/index/active are present on every Tab regardless of
// permissions; url/title are omitted entirely by both browsers unless the extension holds
// "tabs" or a matching host permission, so they're included (when present) via
// TAB_PROJECT_KEYS. favIconUrl is deliberately NOT part of the projection — it's another
// permission-gated, engine-specific URL no corpus entry has needed to diff on yet; add it
// here if one does.
const TAB_PROJECT_KEYS = ["url", "title", "status", "index", "active"] as const;
const looksLikeTab = (o: Record<string, unknown>): boolean =>
  "id" in o && "windowId" in o && "index" in o && typeof o.active === "boolean";
// `temporary` only ever appears on runtime.onInstalled's details object, and only because
// the Firefox driver always installs the .xpi via installAddon(path, true) (temporary=true)
// — the only way to load an unsigned build for testing. A real signed install never sets it.
// It's harness noise, not extension/converter behavior, so it's stripped — but only from
// this one event's first-arg object, not any key named "temporary" at any depth anywhere.
const ONINSTALLED_FIRED = "runtime.onInstalled:fired";

// Bare 10-digit integers (e.g. Chrome's own tab ids, like 1141107017) are indistinguishable
// in magnitude from a 10-digit seconds-epoch timestamp, so only an unambiguous case is
// scrubbed: an exact 13-digit integer (ms-epoch), or any fractional number in the
// seconds-to-ms epoch magnitude range (10-13 integer digits) — no legitimate id or counter is
// ever fractional, so a fractional value that size is safely a timestamp
// (e.g. Tab.lastAccessed: 1785383263679.716). The string-form EPOCH regex above is unchanged
// and still scrubs 10/13-digit epoch numbers embedded in strings (URLs, etc.), where the
// surrounding context already disambiguates them from a bare id.
function isEpochLike(n: number): boolean {
  if (!Number.isFinite(n)) return false;
  const digits = Math.floor(Math.abs(n)).toString().length;
  if (Number.isInteger(n)) return digits === 13;
  return digits >= 10 && digits <= 13;
}

export function normalizeTrace(events: TraceEvent[]): NormalizedEvent[] {
  const idMap = new Map<number, string>();
  // frameId:0 (top frame) and tabId:-1 ("no tab", e.g. devtools) are WebExtensions spec
  // sentinels, not dynamic per-instance ids — remapping them through the per-trace idMap
  // consumed an <id:N> slot (usually the very first one, since they appear on so many
  // events) and skewed every real id that followed. They're already directly comparable
  // across browsers as literal 0 / -1, so they're left untouched instead.
  const mapId = (n: number): number | string => {
    if (n <= 0) return n;
    if (!idMap.has(n)) idMap.set(n, `<id:${idMap.size + 1}>`);
    return idMap.get(n)!;
  };
  const walk = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(walk);
    if (v && typeof v === "object") {
      const src = v as Record<string, unknown>;
      if (looksLikeTab(src)) {
        const o: Record<string, unknown> = {};
        for (const k of TAB_PROJECT_KEYS) if (k in src) o[k] = walk(src[k]);
        return o;
      }
      const o: Record<string, unknown> = {};
      for (const [k, val] of Object.entries(src)) {
        o[k] = ID_KEY.test(k) && typeof val === "number" ? mapId(val) : walk(val);
      }
      return o;
    }
    if (typeof v === "number") return isEpochLike(v) ? "<time>" : v;
    if (typeof v === "string") return v.replace(EXT_URL, "<ext>").replace(ISO, "<time>").replace(EPOCH, "<time>");
    return v;
  };
  const dropTemporary = (o: Record<string, unknown>): Record<string, unknown> => {
    const { temporary: _drop, ...rest } = o;
    return rest;
  };
  return events.map((e) => {
    let args: unknown = e.args;
    if (e.api === ONINSTALLED_FIRED && Array.isArray(args) && args[0] && typeof args[0] === "object" && !Array.isArray(args[0])) {
      args = [dropTemporary(args[0] as Record<string, unknown>), ...args.slice(1)];
    }
    const argsOut = TABS_FIRED.test(e.api) && Array.isArray(args)
      ? args.map((el) => (typeof el === "number" ? mapId(el) : walk(el)))
      : walk(args);
    return { ctx: e.ctx, api: e.api, args: JSON.stringify(argsOut) };
  });
}

function lcsKeep(a: string[], b: string[]): boolean[][] {
  const n = a.length, m = b.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
  const inA = new Array(n).fill(false), inB = new Array(m).fill(false);
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) { inA[i] = true; inB[j] = true; i++; j++; }
    else if (dp[i + 1][j] >= dp[i][j + 1]) i++;
    else j++;
  }
  return [inA, inB];
}

// allowed_diffs entries support an optional "<api-glob>#<substring>" form: the pattern only
// allows a divergence when the glob matches the event's api AND the event's normalized args
// string contains the substring after "#". Plain entries (no "#") keep the old api-only
// behavior. This lets broad-surface-area APIs (runtime.error, net.fetch, runtime.sendMessage)
// be pinned to the one specific call site a triage actually investigated, instead of
// allowlisting every call to that API for the whole corpus entry.
function makeIsAllowed(patterns: string[]): (e: NormalizedEvent) => boolean {
  if (!patterns.length) return () => false;
  const compiled = patterns.map((p) => {
    const hashIdx = p.indexOf("#");
    if (hashIdx === -1) return { match: picomatch(p), substr: null as string | null };
    return { match: picomatch(p.slice(0, hashIdx)), substr: p.slice(hashIdx + 1) };
  });
  return (e: NormalizedEvent) => compiled.some(({ match, substr }) => match(e.api) && (substr === null || e.args.includes(substr)));
}

export function diffTraces(a: NormalizedEvent[], b: NormalizedEvent[], allowedDiffs: string[]): Divergence[] {
  const isAllowed = makeIsAllowed(allowedDiffs);
  const out: Divergence[] = [];
  const ctxs = new Set([...a, ...b].map((e) => e.ctx));
  for (const ctx of ctxs) {
    const ea = a.filter((e) => e.ctx === ctx);
    const eb = b.filter((e) => e.ctx === ctx);
    const key = (e: NormalizedEvent) => `${e.api} ${e.args}`;
    const [inA, inB] = lcsKeep(ea.map(key), eb.map(key));
    ea.forEach((e, i) => { if (!inA[i]) out.push({ side: "a", event: e, allowed: isAllowed(e) }); });
    eb.forEach((e, i) => { if (!inB[i]) out.push({ side: "b", event: e, allowed: isAllowed(e) }); });
  }
  return out;
}
