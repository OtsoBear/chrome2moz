import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import AdmZip from "adm-zip";
import type { Side } from "./telemetry.js";

const SHIM_SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "shim", "shim.js");
const SHIM_NAME = "__c2m_shim.js";
const BG_SHIM_NAME = "__c2m_shim_bg.js";

export function instrumentExtension(dir: string, side: Side, port: number): void {
  const template = readFileSync(SHIM_SRC, "utf8")
    .replaceAll("__C2M_SIDE__", side)
    .replaceAll("__C2M_PORT__", String(port));
  // Content scripts and extension HTML pages (popup, options, ...) use the same file names
  // on both sides, so location-based ctx detection is already symmetric there and gets no
  // override. The background entry point does not: Chrome's MV3 service worker and
  // Firefox's converted event-page background load through different file names, so it
  // gets its own shim copy with ctx forced to a canonical "background" label (see shim.js).
  writeFileSync(join(dir, SHIM_NAME), template.replaceAll("__C2M_CTX_OVERRIDE__", ""));

  const manifestPath = join(dir, "manifest.json");
  const m = JSON.parse(readFileSync(manifestPath, "utf8"));

  if (m.background?.service_worker) {
    writeFileSync(join(dir, BG_SHIM_NAME), template.replaceAll("__C2M_CTX_OVERRIDE__", "background"));
    const orig = m.background.service_worker;
    const isModule = m.background.type === "module";
    const wrapper = isModule
      ? `import "./${BG_SHIM_NAME}";\nimport "./${orig}";\n`
      : `importScripts("${BG_SHIM_NAME}", "${orig}");\n`;
    writeFileSync(join(dir, "__c2m_bg.js"), wrapper);
    m.background.service_worker = "__c2m_bg.js";
  } else if (Array.isArray(m.background?.scripts)) {
    writeFileSync(join(dir, BG_SHIM_NAME), template.replaceAll("__C2M_CTX_OVERRIDE__", "background"));
    m.background.scripts.unshift(BG_SHIM_NAME);
  }

  for (const cs of m.content_scripts ?? []) {
    if (Array.isArray(cs.js)) cs.js.unshift(SHIM_NAME);
  }

  const htmlEntries = [
    m.action?.default_popup,
    m.browser_action?.default_popup,
    m.options_ui?.page,
    m.options_page,
    m.sidebar_action?.default_panel,
  ].filter((p): p is string => typeof p === "string");
  for (const rel of htmlEntries) {
    const p = join(dir, rel.split("?")[0]);
    if (!existsSync(p)) continue;
    let html = readFileSync(p, "utf8");
    const tag = `<script src="${SHIM_NAME}"></script>`;
    if (html.includes(tag)) continue;
    if (/<head[^>]*>/i.test(html)) html = html.replace(/<head[^>]*>/i, (h) => h + tag);
    else html = tag + html;
    writeFileSync(p, html);
  }

  m.host_permissions = Array.from(new Set([...(m.host_permissions ?? []), "http://127.0.0.1/*"]));
  writeFileSync(manifestPath, JSON.stringify(m, null, 2));
}

export function zipDir(dir: string, outFile: string): void {
  const zip = new AdmZip();
  zip.addLocalFolder(dir);
  zip.writeZip(outFile);
}
