import picomatch from "picomatch";
import type { TraceEvent } from "./telemetry.js";

export type NormalizedEvent = { ctx: string; api: string; args: string };
export type Divergence = { side: "a" | "b"; event: NormalizedEvent; allowed: boolean };

const ID_KEY = /^(tabId|windowId|frameId|requestId|id)$/;
const EXT_URL = /(chrome|moz)-extension:\/\/[a-z0-9-]+/gi;
const EPOCH = /\b1[0-9]{9}(?:[0-9]{3})?\b/g;
const ISO = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[^"\s]*/g;

export function normalizeTrace(events: TraceEvent[]): NormalizedEvent[] {
  const idMap = new Map<number, string>();
  const mapId = (n: number) => {
    if (!idMap.has(n)) idMap.set(n, `<id:${idMap.size + 1}>`);
    return idMap.get(n)!;
  };
  const walk = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(walk);
    if (v && typeof v === "object") {
      const o: Record<string, unknown> = {};
      for (const [k, val] of Object.entries(v)) {
        o[k] = ID_KEY.test(k) && typeof val === "number" ? mapId(val) : walk(val);
      }
      return o;
    }
    if (typeof v === "string") return v.replace(EXT_URL, "<ext>").replace(ISO, "<time>").replace(EPOCH, "<time>");
    return v;
  };
  return events.map((e) => ({
    ctx: e.ctx,
    api: e.api,
    args: JSON.stringify(walk(e.args)),
  }));
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

export function diffTraces(a: NormalizedEvent[], b: NormalizedEvent[], allowedDiffs: string[]): Divergence[] {
  const isAllowed = allowedDiffs.length ? picomatch(allowedDiffs) : () => false;
  const out: Divergence[] = [];
  const ctxs = new Set([...a, ...b].map((e) => e.ctx));
  for (const ctx of ctxs) {
    const ea = a.filter((e) => e.ctx === ctx);
    const eb = b.filter((e) => e.ctx === ctx);
    const key = (e: NormalizedEvent) => `${e.api}`;
    const [inA, inB] = lcsKeep(ea.map(key), eb.map(key));
    ea.forEach((e, i) => { if (!inA[i]) out.push({ side: "a", event: e, allowed: isAllowed(e.api) }); });
    eb.forEach((e, i) => { if (!inB[i]) out.push({ side: "b", event: e, allowed: isAllowed(e.api) }); });
  }
  return out;
}
