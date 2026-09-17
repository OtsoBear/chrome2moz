// Manual, live-network build step for web snapshots. NEVER run in CI (CI is serve-only; see
// .github/workflows/e2e.yml and the "Web snapshots" section of
// docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md).
//
// For `--only <id>`, discovers the corpus entry's domains (content_scripts.matches +
// host_permissions + extra_domains, via src/domains.ts) and, for each host missing from
// snapshots/index.json, writes a minimal placeholder HTML capture (valid <head>/<body>, a
// title, and a comment noting it's a placeholder) plus its sha256 into index.json. This is
// the minimal deliverable per the plan: an actual live capture (fetching and saving the real
// page) is a further enhancement, not required here -- the placeholder path alone is
// sufficient to unblock content-script injection coverage for an entry.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { loadCorpus, e2eRoot, type CorpusEntry } from "./corpus.js";
import { discoverDomains } from "./domains.js";
import { loadSnapshotIndex, type SnapshotEntry } from "./snapshots.js";

const INDEX_PATH = join(e2eRoot, "snapshots", "index.json");

function placeholderHtml(host: string): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${host}</title>
</head>
<body>
<!-- Placeholder capture written by snapshotBuild.ts for ${host}. Never run in CI; replace
     with a real logged-out-page capture when live recording is implemented. -->
<div id="root"></div>
</body>
</html>
`;
}

function buildOne(entry: CorpusEntry, index: Record<string, SnapshotEntry[]>): boolean {
  let manifest: Record<string, any>;
  try {
    const manifestPath = entry.source.startsWith("local:")
      ? join(e2eRoot, entry.source.slice(6), "manifest.json")
      : join(e2eRoot, ".cache", "crx", `${entry.id}-${entry.version}.manifest.json`);
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch {
    console.error(`${entry.id}: no local manifest.json found to discover domains from (fetch the source first)`);
    return false;
  }
  const hosts = discoverDomains(manifest, entry.extra_domains ?? []);
  if (hosts.length === 0) {
    console.log(`${entry.id}: no domains discovered, nothing to snapshot`);
    return false;
  }
  const existing = new Set((index[entry.id] ?? []).map((e) => e.host));
  const entryDir = join(e2eRoot, "snapshots", entry.id);
  mkdirSync(entryDir, { recursive: true });
  const entries = index[entry.id] ?? [];
  let changed = false;
  for (const host of hosts) {
    if (existing.has(host)) continue;
    const file = `${host}.html`;
    const html = placeholderHtml(host);
    writeFileSync(join(entryDir, file), html);
    const sha256 = createHash("sha256").update(html).digest("hex");
    entries.push({ host, file, sha256 });
    changed = true;
    console.log(`${entry.id}: wrote placeholder snapshot for ${host}`);
  }
  if (changed) index[entry.id] = entries;
  return changed;
}

const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null;
if (!only) {
  console.error("usage: pnpm snapshot --only <corpus-entry-id>");
  process.exit(1);
}
const entries = loadCorpus().filter((e) => e.id === only);
if (entries.length === 0) {
  console.error(`--only ${only}: no matching corpus entry`);
  process.exit(1);
}
const index = existsSync(INDEX_PATH) ? loadSnapshotIndex() : {};
let anyChanged = false;
for (const entry of entries) {
  if (buildOne(entry, index)) anyChanged = true;
}
if (anyChanged) {
  writeFileSync(INDEX_PATH, JSON.stringify(index, null, 2) + "\n");
  console.log(`updated ${INDEX_PATH}`);
} else {
  console.log("no changes");
}
