import { describe, it, expect, beforeEach } from "vitest";
import { mkdtempSync, writeFileSync, readFileSync, mkdirSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { instrumentExtension } from "../src/injector.js";

function makeExt(manifest: object, files: Record<string, string> = {}): string {
  const dir = mkdtempSync(join(tmpdir(), "c2m-inj-"));
  writeFileSync(join(dir, "manifest.json"), JSON.stringify(manifest));
  for (const [name, content] of Object.entries(files)) writeFileSync(join(dir, name), content);
  return dir;
}
const readManifest = (dir: string) => JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));

describe("instrumentExtension", () => {
  it("rewrites a Chrome service worker background through a wrapper", () => {
    const dir = makeExt(
      { manifest_version: 3, background: { service_worker: "bg.js" } },
      { "bg.js": "// bg" },
    );
    instrumentExtension(dir, "chrome-orig", 41999);
    const m = readManifest(dir);
    expect(m.background.service_worker).toBe("__c2m_bg.js");
    const wrapper = readFileSync(join(dir, "__c2m_bg.js"), "utf8");
    expect(wrapper).toContain('importScripts("__c2m_shim_bg.js", "bg.js")');
    expect(existsSync(join(dir, "__c2m_shim_bg.js"))).toBe(true);
    const bgShim = readFileSync(join(dir, "__c2m_shim_bg.js"), "utf8");
    expect(bgShim).toContain('"background"'); // ctx override baked in for the background entry point
  });

  it("unshifts the shim into a Firefox scripts background and content scripts", () => {
    const dir = makeExt(
      {
        manifest_version: 3,
        background: { scripts: ["bg.js"] },
        content_scripts: [{ matches: ["<all_urls>"], js: ["cs.js"] }],
      },
      { "bg.js": "", "cs.js": "" },
    );
    instrumentExtension(dir, "firefox-conv", 41999);
    const m = readManifest(dir);
    expect(m.background.scripts).toEqual(["__c2m_shim_bg.js", "bg.js"]);
    expect(m.content_scripts[0].js).toEqual(["__c2m_shim.js", "cs.js"]);
  });

  it("injects a script tag into the popup and adds the telemetry host permission", () => {
    const dir = makeExt(
      { manifest_version: 3, action: { default_popup: "popup.html" } },
      { "popup.html": "<html><head><title>x</title></head><body></body></html>" },
    );
    instrumentExtension(dir, "chrome-orig", 41999);
    const html = readFileSync(join(dir, "popup.html"), "utf8");
    expect(html).toContain('<head><script src="__c2m_shim.js"></script>');
    expect(readManifest(dir).host_permissions).toContain("http://127.0.0.1/*");
  });

  it("replaces shim placeholders with side and port", () => {
    const dir = makeExt({ manifest_version: 3 });
    instrumentExtension(dir, "firefox-conv", 42123);
    const shim = readFileSync(join(dir, "__c2m_shim.js"), "utf8");
    expect(shim).toContain('"firefox-conv"');
    expect(shim).toContain("42123");
    expect(shim).not.toContain("__C2M_SIDE__");
  });
});
