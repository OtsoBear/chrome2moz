import { describe, it, expect } from "vitest";
import { matchPatternCoversUrl } from "../src/probes.js";

const u = (s: string) => new URL(s);

describe("matchPatternCoversUrl", () => {
  it("<all_urls> matches any http(s) origin", () => {
    expect(matchPatternCoversUrl("<all_urls>", u("http://127.0.0.1:41990/basic.html"))).toBe(true);
    expect(matchPatternCoversUrl("<all_urls>", u("https://example.com/"))).toBe(true);
  });

  it("scheme wildcard (*) is limited to http/https, not arbitrary schemes", () => {
    expect(matchPatternCoversUrl("*://*/*", u("http://example.com/x"))).toBe(true);
    expect(matchPatternCoversUrl("*://*/*", u("https://example.com/x"))).toBe(true);
    expect(matchPatternCoversUrl("*://*/*", u("ftp://example.com/x"))).toBe(false);
    expect(matchPatternCoversUrl("*://*/*", u("file:///etc/passwd"))).toBe(false);
  });

  it("*.host wildcard matches the bare domain and real subdomains", () => {
    expect(matchPatternCoversUrl("https://*.example.com/*", u("https://example.com/x"))).toBe(true);
    expect(matchPatternCoversUrl("https://*.example.com/*", u("https://sub.example.com/x"))).toBe(true);
    expect(matchPatternCoversUrl("https://*.example.com/*", u("https://a.b.example.com/x"))).toBe(true);
  });

  it("*.host wildcard does NOT over-match a lookalike domain (evillive.com vs live.com)", () => {
    expect(matchPatternCoversUrl("https://*.live.com/*", u("https://evillive.com/x"))).toBe(false);
    expect(matchPatternCoversUrl("https://*.live.com/*", u("https://notlive.com/x"))).toBe(false);
    // sanity: the legitimate subdomain form still matches
    expect(matchPatternCoversUrl("https://*.live.com/*", u("https://onenote.live.com/x"))).toBe(true);
  });

  it("path glob (*) matches any sub-path; a path with no glob requires an exact match", () => {
    expect(matchPatternCoversUrl("https://example.com/foo/*", u("https://example.com/foo/bar"))).toBe(true);
    expect(matchPatternCoversUrl("https://example.com/foo/*", u("https://example.com/other"))).toBe(false);
    expect(matchPatternCoversUrl("https://example.com/exact", u("https://example.com/exact"))).toBe(true);
    expect(matchPatternCoversUrl("https://example.com/exact", u("https://example.com/exact2"))).toBe(false);
  });

  it("an exact (non-wildcard) host requires an exact hostname match, not a subdomain or substring", () => {
    expect(matchPatternCoversUrl("https://example.com/*", u("https://example.com/x"))).toBe(true);
    expect(matchPatternCoversUrl("https://example.com/*", u("https://sub.example.com/x"))).toBe(false);
    expect(matchPatternCoversUrl("https://example.com/*", u("https://notexample.com/x"))).toBe(false);
  });

  it("a pattern for a different scheme or host does not match the 127.0.0.1 fixture origin", () => {
    // this is the exact real-world case that motivated the check: OneNote's content_scripts
    // only match onenote.officeapps.live.com, which must not cover our local fixture server
    expect(matchPatternCoversUrl("https://onenote.officeapps.live.com/*", u("http://127.0.0.1:41990/basic.html"))).toBe(false);
  });
});
