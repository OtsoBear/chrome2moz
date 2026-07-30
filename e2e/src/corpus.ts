import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

export type CorpusEntry = {
  id: string;
  name: string;
  source: string; // "cws" | "local:<path relative to e2e/>"
  version?: string;
  allowed_diffs: string[];
  /** allowed_diffs pattern -> human explanation of why it's a by-design (not bug) divergence. */
  _notes?: Record<string, string>;
  quarantined: boolean;
};

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

export function loadCorpus(): CorpusEntry[] {
  return JSON.parse(readFileSync(join(ROOT, "corpus.json"), "utf8")).extensions;
}
export const e2eRoot = ROOT;
