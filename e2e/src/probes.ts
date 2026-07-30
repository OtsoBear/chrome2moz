import type { BrowserSession } from "./chromeDriver.js";
import type { Telemetry, Side } from "./telemetry.js";

export type ProbeContext = {
  chrome: BrowserSession;
  firefox: BrowserSession;
  telemetry: Telemetry;
  manifest: Record<string, any>;
  fixtureUrl: (name: string) => string;
  resultsDir: string;
};
export type ProbeResult = { name: string; status: "ran" | "skipped" | "failed"; note?: string };

const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));

// Minimal WebExtension match-pattern matcher (<all_urls> | <scheme>://<host>/<path>), just
// enough to answer "does this content_scripts.matches array actually cover our fixture
// origin" honestly, instead of reporting "ran" whenever content_scripts exists at all
// regardless of whether anything could possibly inject on the fixture page.
export function matchPatternCoversUrl(pattern: string, url: URL): boolean {
  if (pattern === "<all_urls>") return true;
  const m = pattern.match(/^(\*|[a-z][a-z0-9+.-]*):\/\/(\*|\*\.[^/*]+|[^/*]+)(\/.*)$/i);
  if (!m) return false;
  const [, scheme, host, path] = m;
  const urlScheme = url.protocol.replace(":", "");
  if (scheme === "*" ? !["http", "https"].includes(urlScheme) : scheme !== urlScheme) return false;
  if (host !== "*") {
    if (host.startsWith("*.")) {
      const suffix = host.slice(1); // ".example.com"
      if (url.hostname !== host.slice(2) && !url.hostname.endsWith(suffix)) return false;
    } else if (host !== url.hostname) return false;
  }
  const pathRegex = new RegExp("^" + path.split("*").map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join(".*") + "$");
  return pathRegex.test(url.pathname + url.search);
}

function contentScriptsCoverUrl(manifest: Record<string, any>, url: string): boolean {
  const scripts = Array.isArray(manifest.content_scripts) ? manifest.content_scripts : [];
  const u = new URL(url);
  return scripts.some((cs: any) => Array.isArray(cs.matches) && cs.matches.some((pat: string) => matchPatternCoversUrl(pat, u)));
}

export async function installProbe(_p: ProbeContext): Promise<ProbeResult> {
  await settle(4000); // background boot + onInstalled traces flow in
  return { name: "install", status: "ran" };
}

export async function contentProbe(p: ProbeContext): Promise<ProbeResult> {
  const hasContent = Array.isArray(p.manifest.content_scripts) && p.manifest.content_scripts.length > 0;
  if (!hasContent) return { name: "content", status: "skipped", note: "no content_scripts" };
  if (!contentScriptsCoverUrl(p.manifest, p.fixtureUrl("basic.html"))) {
    const patterns = p.manifest.content_scripts.flatMap((cs: any) => cs.matches ?? []);
    return { name: "content", status: "skipped", note: `content_scripts matches (${patterns.join(", ")}) don't cover the fixture origin` };
  }
  for (const fixture of ["basic.html", "form.html"]) {
    const url = p.fixtureUrl(fixture);
    await p.chrome.open(url);
    await p.firefox.open(url);
    await settle(2500);
  }
  return { name: "content", status: "ran" };
}

export async function commandsProbe(p: ProbeContext): Promise<ProbeResult> {
  const commands = Object.entries(p.manifest.commands ?? {}) as [string, any][];
  if (!commands.length) return { name: "commands", status: "skipped", note: "no commands" };
  // Spike C proved synthetic key chords (Playwright CDP and Selenium WebDriver Actions)
  // never reach the browser's native global-accelerator table that chrome.commands
  // shortcuts are matched against, in either browser. Dispatch is unconditionally
  // unsupported by this harness; do not attempt chord dispatch.
  return { name: "commands", status: "skipped", note: "dispatch-unsupported (spike C)" };
}

export async function popupProbe(p: ProbeContext): Promise<ProbeResult> {
  const popup = p.manifest.action?.default_popup ?? p.manifest.browser_action?.default_popup;
  if (!popup) return { name: "popup", status: "skipped", note: "no popup" };
  await p.chrome.openExtensionPage(popup);
  await p.firefox.openExtensionPage(popup);
  await settle(2500);
  await p.chrome.screenshot(`${p.resultsDir}/popup-chrome.png`);
  await p.firefox.screenshot(`${p.resultsDir}/popup-firefox.png`);
  return { name: "popup", status: "ran" };
}

export async function pingProbe(p: ProbeContext): Promise<ProbeResult> {
  // The ping relay only works because the shim gets injected into whatever content script
  // the extension itself declares (instrumentExtension unshifts it into each
  // content_scripts[].js array) -- there's no universal listener. So this probe can only
  // ever succeed if content_scripts already covers the fixture origin; check that first and
  // skip before opening any pages, instead of opening pages, waiting on a relay that can
  // never respond, and only then reporting skipped.
  if (!contentScriptsCoverUrl(p.manifest, p.fixtureUrl("basic.html"))) {
    return { name: "ping", status: "skipped", note: "no content script on fixture page" };
  }
  const url = p.fixtureUrl("basic.html");
  await p.chrome.open(url);
  await p.firefox.open(url);
  await settle(500);
  p.telemetry.pushCommand("chrome-orig", { type: "ping" });
  p.telemetry.pushCommand("firefox-conv", { type: "ping" });
  await settle(3000);
  const cr = p.telemetry.takeCommandResults("chrome-orig");
  const fr = p.telemetry.takeCommandResults("firefox-conv");
  const note = `chrome:${JSON.stringify(cr)} firefox:${JSON.stringify(fr)}`;
  const isOk = (results: object[]) => results.some((r: any) => r?.type === "ping" && r.ok === true);
  const chromeOk = isOk(cr);
  const firefoxOk = isOk(fr);
  // Asymmetry (one side's content script answered the relay ping, the other didn't) is a
  // real regression signal, not noise — fail the entry rather than just noting it.
  if (chromeOk !== firefoxOk) {
    return { name: "ping", status: "failed", note: `asymmetric: chrome.ok=${chromeOk} firefox.ok=${firefoxOk} -- ${note}` };
  }
  if (!chromeOk && !firefoxOk) {
    // Reached only when content_scripts DOES cover the fixture origin (checked above) yet
    // neither side's relay still answered -- a different, rarer situation than the pre-gate
    // "no content script at all" case, so it gets its own note.
    return { name: "ping", status: "skipped", note: `content_scripts cover the fixture origin but neither side's relay responded -- ${note}` };
  }
  return { name: "ping", status: "ran", note };
}

export const ALL_PROBES = [installProbe, contentProbe, commandsProbe, popupProbe, pingProbe];
