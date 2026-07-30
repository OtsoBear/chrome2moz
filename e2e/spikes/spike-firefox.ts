import firefox from "selenium-webdriver/firefox.js";
import { Builder } from "selenium-webdriver";
import AdmZip from "adm-zip";
import http from "node:http";
import { readFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

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
  .listen(41801);

const manifest = {
  manifest_version: 3,
  name: "C2M Hello FF",
  version: "1.0",
  background: { scripts: ["background.js"] },
  content_scripts: [{ matches: ["http://127.0.0.1/*"], js: ["content.js"] }],
  browser_specific_settings: { gecko: { id: "c2m-hello@test" } },
  permissions: ["storage"],
  host_permissions: ["http://127.0.0.1/*"]
};
const zip = new AdmZip();
zip.addFile("manifest.json", Buffer.from(JSON.stringify(manifest)));
zip.addFile("background.js", Buffer.from('fetch("http://127.0.0.1:41801/from-bg").catch(()=>{});'));
zip.addFile("content.js", Buffer.from('fetch("http://127.0.0.1:41801/from-content").catch(()=>{});'));
const xpi = join(mkdtempSync(join(tmpdir(), "c2m-")), "hello.xpi");
writeFileSync(xpi, zip.toBuffer());

const driver = await new Builder().forBrowser("firefox").setFirefoxOptions(new firefox.Options()).build();
try {
  await (driver as any).installAddon(xpi, true);
  const caps = await driver.getCapabilities();
  const profile = caps.get("moz:profile") as string;
  await driver.get("http://127.0.0.1:41801/page");
  await driver.sleep(3000);
  const prefs = readFileSync(join(profile, "prefs.js"), "utf8");
  const m = prefs.match(/extensions\.webextensions\.uuids.*?"({.*?})\\?"/);
  console.log("uuids pref line found:", !!m);
  console.log("telemetry hits:", hits);
} finally {
  await driver.quit();
  server.close();
}
