export function crxToZip(buf: Buffer): Buffer {
  if (buf.length >= 4 && buf.readUInt32BE(0) === 0x504b0304) return buf; // already ZIP
  if (buf.length < 12 || buf.toString("ascii", 0, 4) !== "Cr24") {
    throw new Error("not a CRX or ZIP file");
  }
  const version = buf.readUInt32LE(4);
  if (version === 3) {
    const headerLen = buf.readUInt32LE(8);
    return buf.subarray(12 + headerLen);
  }
  if (version === 2) {
    const pubLen = buf.readUInt32LE(8);
    const sigLen = buf.readUInt32LE(12);
    return buf.subarray(16 + pubLen + sigLen);
  }
  throw new Error(`unsupported CRX version ${version}`);
}
