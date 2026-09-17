// Spike 3: both browsers proxied through mitmproxy, serving a stored HTML capture under a
// real hostname (https://example.test/), TLS accepted without NSS import, and an extension
// content script (testdata/hello-extension, matches https://example.test/*) injects on the
// served page in both browsers. See spikes/RESULTS.md "## Web snapshots / mitmproxy" for the
// recorded verdict once this passes.
import { chromium } from "playwright";
import { Builder, type WebDriver } from "selenium-webdriver";
import firefox from "selenium-webdriver/firefox.js";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, cpSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import net from "node:net";
import { zipDir } from "../src/injector.js";

const PORT = 41990;

function waitForPort(port: number, timeoutMs = 10000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve_, reject) => {
    const tryConnect = () => {
      const sock = net.connect(port, "127.0.0.1");
      sock.once("connect", () => { sock.end(); resolve_(); });
      sock.once("error", () => {
        sock.destroy();
        if (Date.now() > deadline) reject(new Error(`mitmdump did not open port ${port} in time`));
        else setTimeout(tryConnect, 200);
      });
    };
    tryConnect();
  });
}

async function startMitmdump(): Promise<ChildProcess> {
  const proc = spawn(
    "mitmdump",
    ["-q", "-p", String(PORT), "-s", "snapshots/serve_addon.py", "--set", "upstream_cert=false", "--set", "connection_strategy=lazy"],
    { cwd: resolve("."), env: { ...process.env, C2M_SNAPSHOT_ID: "spike" }, stdio: ["ignore", "pipe", "pipe"] },
  );
  proc.stdout?.on("data", (d) => process.stdout.write(`[mitmdump] ${d}`));
  proc.stderr?.on("data", (d) => process.stderr.write(`[mitmdump] ${d}`));
  await waitForPort(PORT);
  return proc;
}

async function runChromium(): Promise<{ title: string; injected: boolean }> {
  const extDir = resolve("testdata/hello-extension");
  const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-snap-chrome-")), {
    headless: false, // matches chromeDriver.ts: --headless=new below is what actually runs headless
    args: [
      "--headless=new",
      `--disable-extensions-except=${extDir}`,
      `--load-extension=${extDir}`,
      "--no-first-run",
      "--ignore-certificate-errors",
    ],
    proxy: { server: `http://127.0.0.1:${PORT}` },
    ignoreHTTPSErrors: true,
  });
  await ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker", { timeout: 15000 }));
  const page = await ctx.newPage();
  await page.goto("https://example.test/", { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(1000);
  const title = await page.title();
  const injected = await page.evaluate(() => document.documentElement.dataset.c2mHello === "1");
  await ctx.close();
  return { title, injected };
}

async function runFirefox(): Promise<{ title: string; injected: boolean }> {
  const dir = mkdtempSync(join(tmpdir(), "c2m-snap-firefox-"));
  cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
  const manifestPath = join(dir, "manifest.json");
  const m = JSON.parse(readFileSync(manifestPath, "utf8"));
  m.background = { scripts: ["background.js"] };
  m.browser_specific_settings = { gecko: { id: "c2m-hello-snap@test" } };
  writeFileSync(manifestPath, JSON.stringify(m));
  const xpi = join(dir, "..", "hello-snap.xpi");
  zipDir(dir, xpi);

  const opts = new firefox.Options();
  opts.addArguments("-headless");
  opts.setPreference("network.proxy.type", 1);
  opts.setPreference("network.proxy.http", "127.0.0.1");
  opts.setPreference("network.proxy.http_port", PORT);
  opts.setPreference("network.proxy.ssl", "127.0.0.1");
  opts.setPreference("network.proxy.ssl_port", PORT);
  opts.setPreference("network.proxy.allow_hijacking_localhost", true);
  opts.setPreference("network.proxy.no_proxies_on", "");
  opts.setAcceptInsecureCerts(true);
  const service = new firefox.ServiceBuilder().addArguments("--allow-system-access");
  const driver: WebDriver = await new Builder()
    .forBrowser("firefox")
    .setFirefoxOptions(opts)
    .setFirefoxService(service)
    .build();
  try {
    await (driver as unknown as { installAddon(p: string, temp: boolean): Promise<void> }).installAddon(xpi, true);
    await driver.sleep(1000);
    await driver.get("https://example.test/");
    await driver.sleep(1000);
    const title = await driver.getTitle();
    const injected = await driver.executeScript<boolean>(
      "return document.documentElement.dataset.c2mHello === '1';",
    );
    return { title, injected };
  } finally {
    await driver.quit().catch(() => {});
  }
}

const mitm = await startMitmdump();
try {
  console.log("=== chromium ===");
  console.log(await runChromium());
  console.log("=== firefox ===");
  console.log(await runFirefox());
} finally {
  mitm.kill();
}
