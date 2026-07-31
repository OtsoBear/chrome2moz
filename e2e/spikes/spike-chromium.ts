import { chromium } from "playwright";
import http from "node:http";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const hits: string[] = [];
const server = http
  .createServer((req, res) => {
    if (req.url === "/page") {
      res.setHeader("content-type", "text/html");
      res.end("<html><body>fixture</body></html>");
      return;
    }
    hits.push(req.url ?? "");
    res.setHeader("access-control-allow-origin", "*");
    res.end("ok");
  })
  .listen(41800);

const extDir = resolve("testdata/hello-extension");
// For the spike, temporarily add this to background.js first line:
//   fetch("http://127.0.0.1:41800/from-bg").catch(() => {});
// and to content.js:
//   fetch("http://127.0.0.1:41800/from-content").catch(() => {});
const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-")), {
  headless: false,
  args: [`--disable-extensions-except=${extDir}`, `--load-extension=${extDir}`, "--no-first-run"],
});
const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker"));
console.log("extension id:", new URL(sw.url()).host);
const page = await ctx.newPage();
await page.goto("http://127.0.0.1:41800/page");
await page.waitForTimeout(2000);
console.log("telemetry hits:", hits);
await ctx.close();
server.close();
