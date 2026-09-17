"""mitmproxy serve-addon: returns a stored HTML capture for each snapshotted host, offline.
Selected by the C2M_SNAPSHOT_ID env var (the corpus entry id). Document requests to a
snapshotted host get the stored HTML; any other request to that host gets an empty 200 so
subresource loads do not hang. Hosts not in the snapshot are refused (no live network),
EXCEPT loopback hosts, which are always passed through untouched: the harness's own
fixture/telemetry servers (src/fixtureServer.ts, src/telemetry.ts) run on 127.0.0.1 in the
same test process, not on the live internet, and the browsers under test are proxied
browser-wide (not just for the snapshotted host), so the shim's telemetry POSTs to
127.0.0.1 would otherwise be silently swallowed by the same refusal that blocks real
internet hosts -- confirmed as a real bug during Task 5 (OneNote's report showed
chrome=0 firefox=0 matched=0 with content:ran, i.e. the page loaded and the content script
injected, but every telemetry POST from the shim was refused by this addon)."""
import json
import os
from mitmproxy import http

BASE = os.path.dirname(__file__)
ENTRY = os.environ.get("C2M_SNAPSHOT_ID", "")
_index = json.load(open(os.path.join(BASE, "index.json")))
_hosts = {}
for e in _index.get(ENTRY, []):
    _hosts[e["host"]] = os.path.join(BASE, ENTRY, e["file"])

LOOPBACK_HOSTS = {"127.0.0.1", "localhost", "::1", "[::1]"}


def request(flow: http.HTTPFlow) -> None:
    host = flow.request.pretty_host
    if host in LOOPBACK_HOSTS:
        return  # let mitmproxy forward to the harness's own local fixture/telemetry servers
    if host not in _hosts:
        flow.response = http.Response.make(204, b"", {})
        return
    accept = flow.request.headers.get("accept", "")
    is_doc = flow.request.method == "GET" and ("text/html" in accept or flow.request.path in ("/", ""))
    if is_doc:
        with open(_hosts[host], "rb") as f:
            body = f.read()
        flow.response = http.Response.make(200, body, {"Content-Type": "text/html; charset=utf-8"})
    else:
        flow.response = http.Response.make(200, b"", {"Content-Type": "application/octet-stream"})
