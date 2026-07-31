import { describe, it, expect } from "vitest";
import AdmZip from "adm-zip";
import { crxToZip } from "../src/crx.js";

function fakeCrx3(zipBuf: Buffer, headerLen = 8): Buffer {
  const head = Buffer.alloc(12 + headerLen);
  head.write("Cr24", 0, "ascii");
  head.writeUInt32LE(3, 4);
  head.writeUInt32LE(headerLen, 8);
  return Buffer.concat([head, zipBuf]);
}

function zipWithManifest(): Buffer {
  const z = new AdmZip();
  z.addFile("manifest.json", Buffer.from(JSON.stringify({ version: "2.1.0" })));
  return z.toBuffer();
}

describe("crxToZip", () => {
  it("strips a CRX3 header", () => {
    const zip = zipWithManifest();
    const out = crxToZip(fakeCrx3(zip));
    expect(out.equals(zip)).toBe(true);
  });
  it("passes through a plain zip", () => {
    const zip = zipWithManifest();
    expect(crxToZip(zip).equals(zip)).toBe(true);
  });
  it("rejects garbage", () => {
    expect(() => crxToZip(Buffer.from("nope"))).toThrow(/not a CRX/);
  });
});
