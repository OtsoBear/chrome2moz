"""mitmproxy serve-addon: returns a stored HTML capture for each snapshotted host, offline.
Selected by the C2M_SNAPSHOT_ID env var (the corpus entry id). Document requests to a
snapshotted host get the stored HTML; any other request to that host gets an empty 200 so
subresource loads do not hang. Hosts not in the snapshot are refused (no live network)."""
import json
import os
from mitmproxy import http

BASE = os.path.dirname(__file__)
ENTRY = os.environ.get("C2M_SNAPSHOT_ID", "")
_index = json.load(open(os.path.join(BASE, "index.json")))
_hosts = {}
for e in _index.get(ENTRY, []):
    _hosts[e["host"]] = os.path.join(BASE, ENTRY, e["file"])


def request(flow: http.HTTPFlow) -> None:
    host = flow.request.pretty_host
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
