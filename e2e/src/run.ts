import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { loadCorpus, e2eRoot, type CorpusEntry } from "./corpus.js";
import { fetchExtension } from "./fetchCrx.js";
import { buildConverter, convert } from "./convert.js";
import { instrumentExtension, zipDir } from "./injector.js";
import { startTelemetry } from "./telemetry.js";
import { startFixtures } from "./fixtureServer.js";
import { launchChrome } from "./chromeDriver.js";
import { launchFirefox } from "./firefoxDriver.js";
import { normalizeTrace, diffTraces } from "./diff.js";
import { ALL_PROBES, type ProbeResult } from "./probes.js";

const TELEMETRY_PORT = 41999;
const FIXTURE_PORT = 41990;

type ExtReport = {
  id: string; name: string; quarantined: boolean;
  probes: ProbeResult[];
  divergences: { side: string; api: string; args: string; ctx: string; allowed: boolean }[];
  pass: boolean;
};

async function prepareSource(entry: CorpusEntry, work: string): Promise<string> {
  const src = join(work, "source");
  if (entry.source.startsWith("local:")) {
    cpSync(join(e2eRoot, entry.source.slice(6)), src, { recursive: true });
  } else {
    const { zipPath } = await fetchExtension(entry.id, entry.version ?? null, join(e2eRoot, ".cache", "crx"));
    new AdmZip(zipPath).extractAllTo(src, true);
  }
  if (!existsSync(join(src, "manifest.json"))) throw new Error(`${entry.id}: no manifest.json at source root`);
  return src;
}

async function runOne(entry: CorpusEntry): Promise<ExtReport> {
  const work = join(e2eRoot, "results", entry.id);
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });

  const source = await prepareSource(entry, work);
  const manifest = JSON.parse(readFileSync(join(source, "manifest.json"), "utf8"));

  const convertedDir = convert(source, join(work, "converted"));
  const convManifest = JSON.parse(readFileSync(join(convertedDir, "manifest.json"), "utf8"));
  const geckoId: string = convManifest.browser_specific_settings?.gecko?.id;
  if (!geckoId) throw new Error(`${entry.id}: converted build has no gecko id`);

  const chromeDir = join(work, "chrome-instrumented");
  cpSync(source, chromeDir, { recursive: true });
  instrumentExtension(chromeDir, "chrome-orig", TELEMETRY_PORT);
  instrumentExtension(convertedDir, "firefox-conv", TELEMETRY_PORT);
  const xpi = join(work, "converted.xpi");
  zipDir(convertedDir, xpi);

  const telemetry = await startTelemetry(TELEMETRY_PORT);
  const fixtures = await startFixtures(FIXTURE_PORT);
  const probes: ProbeResult[] = [];
  let chrome, firefox;
  try {
    chrome = await launchChrome(chromeDir);
    firefox = await launchFirefox(xpi, geckoId);
    for (const probe of ALL_PROBES) {
      probes.push(await probe({ chrome, firefox, telemetry, manifest, fixtureUrl: fixtures.url, resultsDir: work }));
    }
  } finally {
    await chrome?.close().catch(() => {});
    await firefox?.close().catch(() => {});
    await fixtures.close();
  }

  const a = normalizeTrace(telemetry.getEvents("chrome-orig"));
  const b = normalizeTrace(telemetry.getEvents("firefox-conv"));
  await telemetry.close();
  const divergences = diffTraces(a, b, entry.allowed_diffs).map((d) => ({
    side: d.side === "a" ? "chrome-only" : "firefox-only",
    api: d.event.api, args: d.event.args, ctx: d.event.ctx, allowed: d.allowed,
  }));

  const report: ExtReport = {
    id: entry.id, name: entry.name, quarantined: entry.quarantined, probes, divergences,
    // A probe can fail outright (e.g. pingProbe's chrome/firefox asymmetry) independent of
    // any traced divergence — that's a regression signal in its own right.
    pass: divergences.every((d) => d.allowed) && probes.every((p) => p.status !== "failed"),
  };
  writeFileSync(join(work, "report.json"), JSON.stringify(report, null, 2));
  writeFileSync(join(work, "trace-chrome.json"), JSON.stringify(a, null, 2));
  writeFileSync(join(work, "trace-firefox.json"), JSON.stringify(b, null, 2));
  return report;
}

const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null;
buildConverter();
const entries = loadCorpus().filter((e) => !only || e.id === only);
let failed = false;
for (const entry of entries) {
  let report = await runOne(entry);
  if (!report.pass && !entry.quarantined) {
    console.log(`↻ ${entry.name}: divergence found, retrying once to confirm...`);
    report = await runOne(entry); // flake control: must reproduce
  }
  const mark = report.pass ? "PASS" : entry.quarantined ? "FAIL (quarantined)" : "FAIL";
  const probeSummary = report.probes
    .map((p) => `${p.name}:${p.status}${p.note ? ` (${p.note.slice(0, 150)})` : ""}`)
    .join(" ");
  console.log(`${mark}  ${entry.name}  probes: ${probeSummary}  unallowed divergences: ${report.divergences.filter((d) => !d.allowed).length}`);
  for (const d of report.divergences.filter((x) => !x.allowed).slice(0, 20)) {
    console.log(`   ${d.side} [${d.ctx}] ${d.api} ${d.args.slice(0, 120)}`);
  }
  if (!report.pass && !entry.quarantined) failed = true;
}
process.exit(failed ? 1 : 0);
