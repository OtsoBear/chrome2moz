// Domain discovery for web snapshots: the hosts an extension actually targets, so a snapshot
// can be served under the real hostname and the extension's content scripts inject.
const HOST_RE = /^(?:\*|[a-z][a-z0-9+.-]*):\/\/([^/*]+|\*\.[^/*]+)(?:\/.*)?$/i;

function hostOf(pattern: string): string | null {
  if (pattern === "<all_urls>") return null;
  const m = pattern.match(HOST_RE);
  if (!m) return null;
  let host = m[1];
  if (host === "*") return null; // wildcard-only host, keep to standard fixtures
  if (host.startsWith("*.")) host = host.slice(2); // *.example.com -> example.com
  return host;
}

export function discoverDomains(manifest: Record<string, any>, extraDomains: string[] = []): string[] {
  const out = new Set<string>();
  const patterns: string[] = [];
  for (const cs of manifest.content_scripts ?? []) for (const p of cs.matches ?? []) patterns.push(p);
  for (const p of manifest.host_permissions ?? []) patterns.push(p);
  for (const p of patterns) { const h = hostOf(p); if (h) out.add(h); }
  for (const d of extraDomains) out.add(d);
  return [...out].slice(0, 20);
}
