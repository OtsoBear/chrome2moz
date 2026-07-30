import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { startTelemetry, type Telemetry } from "../src/telemetry.js";

let t: Telemetry;
const base = "http://127.0.0.1:41999";

beforeAll(async () => { t = await startTelemetry(41999); });
afterAll(async () => { await t.close(); });

describe("telemetry server", () => {
  it("collects posted trace events by side", async () => {
    await fetch(`${base}/trace`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ side: "chrome-orig", events: [{ seq: 0, ctx: "background", api: "storage.local.set", args: [{}] }] }),
    });
    expect(t.getEvents("chrome-orig")).toHaveLength(1);
    expect(t.getEvents("firefox-conv")).toHaveLength(0);
  });
  it("serves queued commands once, then 204", async () => {
    t.pushCommand("chrome-orig", { type: "ping" });
    const r1 = await fetch(`${base}/cmd?side=chrome-orig`);
    expect(r1.status).toBe(200);
    expect(await r1.json()).toEqual({ type: "ping" });
    const r2 = await fetch(`${base}/cmd?side=chrome-orig`);
    expect(r2.status).toBe(204);
  });
  it("collects command results", async () => {
    await fetch(`${base}/cmdresult`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ side: "firefox-conv", result: { type: "ping", ok: true } }),
    });
    expect(t.takeCommandResults("firefox-conv")).toEqual([{ type: "ping", ok: true }]);
  });
  it("answers CORS preflight", async () => {
    const r = await fetch(`${base}/trace`, { method: "OPTIONS" });
    expect(r.status).toBe(204);
    expect(r.headers.get("access-control-allow-origin")).toBe("*");
  });
});
