import http from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { join, dirname, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const FIXTURES = join(dirname(fileURLToPath(import.meta.url)), "..", "fixtures");

export function startFixtures(port: number): Promise<{ url(name: string): string; close(): Promise<void> }> {
  const server = http.createServer((req, res) => {
    const name = normalize(req.url ?? "/").replace(/^([/\\.])+/, "");
    const p = join(FIXTURES, name || "basic.html");
    if (!p.startsWith(FIXTURES) || !existsSync(p)) { res.writeHead(404); res.end(); return; }
    res.setHeader("content-type", "text/html");
    res.end(readFileSync(p));
  });
  return new Promise((resolve) =>
    server.listen(port, "127.0.0.1", () =>
      resolve({
        url: (name) => `http://127.0.0.1:${port}/${name}`,
        close: () => new Promise((r) => server.close(() => r())),
      }),
    ),
  );
}
