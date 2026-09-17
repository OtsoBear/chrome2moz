import { describe, it, expect } from "vitest";
import { discoverDomains } from "../src/domains.js";

describe("discoverDomains", () => {
  it("extracts hosts from content_scripts.matches and host_permissions", () => {
    const m = {
      content_scripts: [{ matches: ["https://onenote.officeapps.live.com/*"] }],
      host_permissions: ["https://www.onenote.com/*"],
    };
    expect(discoverDomains(m).sort()).toEqual(["onenote.officeapps.live.com", "www.onenote.com"]);
  });

  it("drops <all_urls> and wildcard-only host patterns", () => {
    const m = { host_permissions: ["<all_urls>", "*://*/*", "https://x.example.com/*"] };
    expect(discoverDomains(m)).toEqual(["x.example.com"]);
  });

  it("de-wildcards a leading *. and de-dupes, capping at 20", () => {
    const m = { content_scripts: [{ matches: ["https://*.example.com/*", "https://example.com/*"] }] };
    expect(discoverDomains(m)).toEqual(["example.com"]);
    const many = { host_permissions: Array.from({ length: 30 }, (_, i) => `https://h${i}.test/*`) };
    expect(discoverDomains(many).length).toBe(20);
  });

  it("includes extra_domains", () => {
    expect(discoverDomains({}, ["manual.test"])).toEqual(["manual.test"]);
  });
});
