import http from "node:http";

export type Side = "chrome-orig" | "firefox-conv" | "firefox-orig";
export type TraceEvent = { seq: number; ctx: string; api: string; args: unknown[] };

export interface Telemetry {
  getEvents(side: Side): TraceEvent[];
  pushCommand(side: Side, cmd: object): void;
  takeCommandResults(side: Side): object[];
  clear(): void;
  close(): Promise<void>;
}

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET,POST,OPTIONS",
  "access-control-allow-headers": "content-type",
};

export function startTelemetry(port: number): Promise<Telemetry> {
  const events = new Map<string, TraceEvent[]>();
  const commands = new Map<string, object[]>();
  const results = new Map<string, object[]>();
  const get = <T>(m: Map<string, T[]>, k: string) => m.get(k) ?? (m.set(k, []), m.get(k)!);

  const server = http.createServer((req, res) => {
    const url = new URL(req.url ?? "/", `http://127.0.0.1:${port}`);
    if (req.method === "OPTIONS") { res.writeHead(204, CORS); res.end(); return; }
    if (req.method === "GET" && url.pathname === "/cmd") {
      const q = get(commands, url.searchParams.get("side") ?? "");
      const cmd = q.shift();
      if (!cmd) { res.writeHead(204, CORS); res.end(); return; }
      res.writeHead(200, { ...CORS, "content-type": "application/json" });
      res.end(JSON.stringify(cmd));
      return;
    }
    if (req.method === "POST") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        try {
          const data = JSON.parse(body);
          if (url.pathname === "/trace") get(events, data.side).push(...data.events);
          else if (url.pathname === "/cmdresult") get(results, data.side).push(data.result);
        } catch { /* malformed posts are dropped */ }
        res.writeHead(200, CORS);
        res.end("ok");
      });
      return;
    }
    res.writeHead(404, CORS);
    res.end();
  });

  return new Promise((resolve) =>
    server.listen(port, "127.0.0.1", () =>
      resolve({
        getEvents: (s) => [...get(events, s)],
        pushCommand: (s, c) => { get(commands, s).push(c); },
        takeCommandResults: (s) => get(results, s).splice(0),
        clear: () => { events.clear(); commands.clear(); results.clear(); },
        close: () => new Promise((r) => server.close(() => r())),
      }),
    ),
  );
}
