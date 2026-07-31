import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { crxToZip } from "./crx.js";

const CRX_URL = (id: string) =>
  `https://clients2.google.com/service/update2/crx?response=redirect&prodversion=126.0&acceptformat=crx3&x=id%3D${id}%26uc`;

export async function fetchExtension(
  id: string,
  pinnedVersion: string | null,
  cacheDir: string,
): Promise<{ zipPath: string; version: string }> {
  mkdirSync(cacheDir, { recursive: true });
  if (pinnedVersion) {
    const cached = join(cacheDir, `${id}-${pinnedVersion}.zip`);
    if (existsSync(cached)) return { zipPath: cached, version: pinnedVersion };
  }
  const res = await fetch(CRX_URL(id), { redirect: "follow" });
  if (!res.ok) throw new Error(`CRX download failed for ${id}: HTTP ${res.status}`);
  const zip = crxToZip(Buffer.from(await res.arrayBuffer()));
  const manifest = JSON.parse(new AdmZip(zip).readAsText("manifest.json"));
  const version: string = manifest.version;
  if (pinnedVersion && version !== pinnedVersion) {
    throw new Error(
      `${id}: CWS serves ${version} but corpus pins ${pinnedVersion} and no cached copy exists. ` +
        `Re-pin the corpus entry or restore the cached archive.`,
    );
  }
  const out = join(cacheDir, `${id}-${version}.zip`);
  writeFileSync(out, zip);
  return { zipPath: out, version };
}
