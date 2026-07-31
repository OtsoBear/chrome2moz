import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { e2eRoot } from "./corpus.js";

const REPO_ROOT = join(e2eRoot, "..");

export function buildConverter(): void {
  execFileSync("cargo", ["build", "--release", "--features", "cli"], { cwd: REPO_ROOT, stdio: "inherit" });
}

/** Converts an unpacked Chrome extension dir; returns dir of the unpacked Firefox build. */
export function convert(inputDir: string, outDir: string): string {
  mkdirSync(outDir, { recursive: true });
  execFileSync(join(REPO_ROOT, "target", "release", "chrome2moz"), ["convert", "-i", inputDir, "-o", outDir, "--yes"], {
    stdio: "inherit",
  });
  if (existsSync(join(outDir, "manifest.json"))) return outDir;
  const xpi = readdirSync(outDir).find((f) => f.endsWith(".xpi") || f.endsWith(".zip"));
  if (!xpi) throw new Error(`converter produced neither manifest.json nor an .xpi in ${outDir}`);
  const unpacked = join(outDir, "unpacked");
  new AdmZip(join(outDir, xpi)).extractAllTo(unpacked, true);
  return unpacked;
}
