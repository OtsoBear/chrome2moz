import { chromium } from "playwright";
import firefox from "selenium-webdriver/firefox.js";
import { Builder, By, Key } from "selenium-webdriver";
import AdmZip from "adm-zip";
import http from "node:http";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Spike C: does synthetically dispatched keyboard input trigger
// chrome.commands.onCommand in Chromium and Firefox?
//
// macOS caveat: the hello-extension manifest declares suggested_key
// "Ctrl+Shift+9". On macOS, Chrome maps that default binding to
// Command+Shift+9 (MacCtrl -> Command unless "mac" override is given), so
// both Control+Shift+9 and Meta+Shift+9 are tried on each browser before
// concluding a chord "doesn't fire".

async function runChromium(): Promise<{ fired: boolean; chord: string | null }> {
  const extDir = resolve("testdata/hello-extension");
  const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-")), {
    headless: false,
    args: [`--disable-extensions-except=${extDir}`, `--load-extension=${extDir}`, "--no-first-run"],
  });
  try {
    const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker"));
    const page = await ctx.newPage();
    await page.goto("https://example.com");

    for (const chord of ["Control+Shift+9", "Meta+Shift+9"]) {
      await page.keyboard.press(chord);
      await page.waitForTimeout(1000);
      const stored = (await sw.evaluate(() => chrome.storage.local.get("lastCommand"))) as {
        lastCommand?: string;
      };
      console.log(`chromium onCommand fired after ${chord}:`, stored);
      if (stored.lastCommand) {
        return { fired: true, chord };
      }
    }
    return { fired: false, chord: null };
  } finally {
    await ctx.close();
  }
}

async function runFirefox(): Promise<{ fired: boolean; chord: string | null }> {
  const hits: string[] = [];
  const server = http
    .createServer((req, res) => {
      if (req.url === "/page") {
        res.setHeader("content-type", "text/html");
        res.end("<html><body>fixture</body></html>");
        return;
      }
      if (req.url?.startsWith("/cmd-fired")) {
        hits.push(req.url);
      }
      res.setHeader("access-control-allow-origin", "*");
      res.end("ok");
    })
    .listen(41802);

  // Firefox has no service-worker `evaluate` readback (no SW in MV3 event
  // pages there), so instead of reading storage directly we use a
  // content-script relay: background sets chrome.storage.local.lastCommand
  // on commands.onCommand, a content script listens for storage.onChanged
  // and fetches /cmd-fired?cmd=<name> against our localhost server, and we
  // just check the server's hit log. Simpler than the brief's sketch (no
  // extra message-passing layer) and it works.
  const manifest = {
    manifest_version: 3,
    name: "C2M Hello FF Commands",
    version: "1.0",
    background: { scripts: ["background.js"] },
    content_scripts: [{ matches: ["http://127.0.0.1/*"], js: ["content.js"] }],
    browser_specific_settings: { gecko: { id: "c2m-hello-cmd@test" } },
    permissions: ["storage"],
    host_permissions: ["http://127.0.0.1/*"],
    commands: {
      "hello-command": { suggested_key: { default: "Ctrl+Shift+9" }, description: "test command" },
    },
  };
  const zip = new AdmZip();
  zip.addFile("manifest.json", Buffer.from(JSON.stringify(manifest)));
  zip.addFile(
    "background.js",
    Buffer.from(
      'chrome.commands.onCommand.addListener((cmd) => { chrome.storage.local.set({ lastCommand: cmd }); });'
    )
  );
  zip.addFile(
    "content.js",
    Buffer.from(
      'chrome.storage.onChanged.addListener((changes, area) => { ' +
        'if (area === "local" && changes.lastCommand) { ' +
        'fetch("http://127.0.0.1:41802/cmd-fired?cmd=" + encodeURIComponent(changes.lastCommand.newValue)).catch(() => {}); ' +
        '} });'
    )
  );
  const xpi = join(mkdtempSync(join(tmpdir(), "c2m-")), "hello-cmd.xpi");
  writeFileSync(xpi, zip.toBuffer());

  const driver = await new Builder().forBrowser("firefox").setFirefoxOptions(new firefox.Options()).build();
  try {
    await (driver as any).installAddon(xpi, true);
    await driver.get("http://127.0.0.1:41802/page");
    await driver.sleep(1000);
    // Give the page (and therefore the content script's window) input
    // focus before sending key chords via Actions.
    await driver.findElement(By.tagName("body")).click();

    for (const [label, mod] of [
      ["Control+Shift+9", Key.CONTROL],
      ["Meta+Shift+9", Key.META],
    ] as const) {
      await driver.actions().keyDown(mod).keyDown(Key.SHIFT).sendKeys("9").keyUp(Key.SHIFT).keyUp(mod).perform();
      await driver.sleep(1500);
      console.log(`firefox cmd-fired hits after ${label}:`, hits);
      if (hits.length > 0) {
        return { fired: true, chord: label };
      }
    }
    return { fired: false, chord: null };
  } finally {
    await driver.quit();
    server.close();
  }
}

const chromiumResult = await runChromium();
console.log("=== chromium verdict ===", chromiumResult);

const firefoxResult = await runFirefox();
console.log("=== firefox verdict ===", firefoxResult);
