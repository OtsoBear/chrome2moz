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
export type ProbeResult = { name: string; status: "ran" | "skipped"; note?: string };

const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function installProbe(_p: ProbeContext): Promise<ProbeResult> {
  await settle(4000); // background boot + onInstalled traces flow in
  return { name: "install", status: "ran" };
}

export async function contentProbe(p: ProbeContext): Promise<ProbeResult> {
  const hasContent = Array.isArray(p.manifest.content_scripts) && p.manifest.content_scripts.length > 0;
  if (!hasContent) return { name: "content", status: "skipped", note: "no content_scripts" };
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
  const url = p.fixtureUrl("basic.html");
  await p.chrome.open(url);
  await p.firefox.open(url);
  await settle(500);
  p.telemetry.pushCommand("chrome-orig", { type: "ping" });
  p.telemetry.pushCommand("firefox-conv", { type: "ping" });
  await settle(3000);
  const cr = p.telemetry.takeCommandResults("chrome-orig");
  const fr = p.telemetry.takeCommandResults("firefox-conv");
  return { name: "ping", status: "ran", note: `chrome:${JSON.stringify(cr)} firefox:${JSON.stringify(fr)}` };
}

export const ALL_PROBES = [installProbe, contentProbe, commandsProbe, popupProbe, pingProbe];
