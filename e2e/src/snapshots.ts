// Web snapshot index + mitmdump lifecycle. Serves a stored HTML capture under the real
// hostname an extension targets, offline (no live network at run time). See
// spikes/RESULTS.md "## Web snapshots / mitmproxy" for the confirmed mitmdump flags.
import { spawn, type ChildProcess } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import net from "node:net";
import { e2eRoot } from "./corpus.js";

export type SnapshotEntry = { host: string; file: string; sha256: string };

const INDEX_PATH = join(e2eRoot, "snapshots", "index.json");

export function loadSnapshotIndex(): Record<string, SnapshotEntry[]> {
  return JSON.parse(readFileSync(INDEX_PATH, "utf8"));
}

export function hasSnapshot(id: string): boolean {
  const index = loadSnapshotIndex();
  return Array.isArray(index[id]) && index[id].length > 0;
}

export function snapshotUrls(id: string): string[] {
  const index = loadSnapshotIndex();
  return (index[id] ?? []).map((e) => `https://${e.host}/`);
}

function waitForPort(port: number, timeoutMs = 10000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const tryConnect = () => {
      const sock = net.connect(port, "127.0.0.1");
      sock.once("connect", () => { sock.end(); resolve(); });
      sock.once("error", () => {
        sock.destroy();
        if (Date.now() > deadline) reject(new Error(`mitmdump did not open port ${port} in time`));
        else setTimeout(tryConnect, 200);
      });
    };
    tryConnect();
  });
}

export async function startSnapshotServer(
  id: string,
  port: number,
): Promise<{ proxyServer: string; close(): Promise<void> }> {
  const proc: ChildProcess = spawn(
    "mitmdump",
    [
      "-q",
      "-p", String(port),
      "-s", join(e2eRoot, "snapshots", "serve_addon.py"),
      "--set", "upstream_cert=false",
      "--set", "connection_strategy=lazy",
    ],
    { cwd: e2eRoot, env: { ...process.env, C2M_SNAPSHOT_ID: id }, stdio: ["ignore", "pipe", "pipe"] },
  );
  proc.stdout?.on("data", (d) => process.stdout.write(`[mitmdump:${id}] ${d}`));
  proc.stderr?.on("data", (d) => process.stderr.write(`[mitmdump:${id}] ${d}`));
  await waitForPort(port);
  return {
    proxyServer: `http://127.0.0.1:${port}`,
    async close() {
      proc.kill();
    },
  };
}
