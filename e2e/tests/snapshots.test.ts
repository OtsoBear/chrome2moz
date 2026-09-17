import { describe, it, expect } from "vitest";
import { loadSnapshotIndex, hasSnapshot } from "../src/snapshots.js";

describe("snapshot index", () => {
  it("parses the seed index", () => {
    const index = loadSnapshotIndex();
    expect(index.spike).toEqual([{ host: "example.test", file: "example.test.html", sha256: "" }]);
  });

  it("hasSnapshot is true for a seeded entry, false otherwise", () => {
    expect(hasSnapshot("spike")).toBe(true);
    expect(hasSnapshot("nope")).toBe(false);
  });
});
