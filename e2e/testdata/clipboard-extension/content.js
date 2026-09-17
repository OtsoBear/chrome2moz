// Deterministic clipboard round-trip: write a fixed string, read it back. The spy shim
// records clipboard.writeText [text] and clipboard.readText:resolve [value] on both browsers;
// with permissions pre-granted (harness drivers) both resolve, so the two traces MATCH.
(async () => {
  const MARK = "c2m-clipboard-gate";
  try {
    await navigator.clipboard.writeText(MARK);
    await navigator.clipboard.readText();
  } catch (e) {
    // Recorded as clipboard.*:reject on both sides symmetrically if permissions somehow fail.
  }
})();
