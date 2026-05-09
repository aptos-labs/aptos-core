#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Build the per-validator failure metrics file used by the mainnet-mirror snapshot
script to assign each validator an availability_class (Healthy / OnlineButFlaky
/ StableChronic / EpisodicSpike).

We compute per-validator FAILURE RATE (a fraction in [0, 1]) — the fraction of
this validator's leader rounds that ended in `failed_proposer_indices`:

    rate = aptos_failed_proposals_in_window /
           (aptos_failed_proposals_in_window + aptos_committed_proposals_in_window)

NOTE on semantics: prior versions stored the raw `aptos_failed_proposals_in_window`
gauge AVERAGED over 7d. That gauge is a COUNT of failed proposals in the rolling
LR window (default ~1070 blocks ≈ 10 leader rounds per validator on mainnet) —
NOT a fraction. Treating it as a percentage in the forge failpoint code
(`(fp * 100).round() as %`) over-modeled chronic failure by ~10× (apne1-0's
real rate is ~3% per leader round, but its old `fp_7d_avg=0.304` × 100 → 30%
which we used as the failpoint rate). Switching to the rate fraction makes the
field's value mean what its name suggests.

Two well-known data-quality issues we still handle:

  1. Multi-source duplication. Aptos Labs validators report via the cluster
     vmagent (`metrics_source="vmagent"`, kubernetes_pod_name like
     `mainnet-validator-NAME-aptos-node-0-validator-0`). Community validators
     report via the telemetry-service (`metrics_source="telemetry-service"`,
     kubernetes_pod_name like `peer_id:NAME//0xHEX`). Some labs validators
     appear in BOTH (with the same peer_id). We aggregate by peer_id taking
     the max across sources.

  2. Stale run_uuids. The telemetry-service partitions each peer_id into one
     time series per `run_uuid` (a session id assigned at validator start).
     When a validator restarts the old uuid keeps appearing in queries with
     its last-known value indefinitely, even though that instance is gone.
     Mitigation: `avg by (kubernetes_pod_name)` aggregates across run_uuids,
     dampening but not fully filtering the stale signal.

Usage:
    GRAFANA_TOKEN=... python3 scripts/pull_mainnet_failure_metrics.py

Output: testsuite/testcases/src/data/mainnet-mirror/failure_metrics.json
keyed by peer_id, each entry holding {fp_7d_avg, fp_30d_max} where both fields
are now FAIL RATES (fractions in [0,1]), e.g. fp_7d_avg=0.025 means 2.5% of
this validator's leader rounds failed averaged over 7 days.
"""

import argparse
import json
import os
import re
import ssl
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


GRAFANA_BASE = "https://aptoslabs.grafana.net"
DATASOURCE_UID = "vU-Lwva4k"  # VictoriaMetrics Mainnet US
APTOS_REST = "https://fullnode.mainnet.aptoslabs.com/v1"


def _make_ssl_context():
    try:
        import certifi
        return ssl.create_default_context(cafile=certifi.where())
    except ImportError:
        pass
    if os.path.exists("/etc/ssl/cert.pem"):
        return ssl.create_default_context(cafile="/etc/ssl/cert.pem")
    return ssl.create_default_context()


_SSL_CTX = _make_ssl_context()


def grafana_query(promql, token):
    url = f"{GRAFANA_BASE}/api/datasources/proxy/uid/{DATASOURCE_UID}/api/v1/query"
    body = urllib.parse.urlencode({
        "query": promql,
        "time": str(int(time.time())),
    }).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/x-www-form-urlencoded",
        },
    )
    with urllib.request.urlopen(req, timeout=60, context=_SSL_CTX) as f:
        body = json.load(f)
    if body.get("status") != "success":
        raise RuntimeError(f"Prometheus query failed: {json.dumps(body)[:400]}")
    return body["data"]["result"]


def normalize64(addr):
    h = addr.lower()
    if h.startswith("0x"):
        h = h[2:]
    return "0x" + h.zfill(64)


def fetch_active_validators():
    """Returns list of active peer_ids in their REST API string format
    (with leading zeros possibly stripped)."""
    url = f"{APTOS_REST}/accounts/0x1/resource/0x1::stake::ValidatorSet"
    req = urllib.request.Request(url, headers={"User-Agent": "aptos-forge-failure-metrics/1.0"})
    with urllib.request.urlopen(req, timeout=20, context=_SSL_CTX) as f:
        data = json.load(f)
    return [v["addr"] for v in data["data"]["active_validators"]]


def extract_peer_id(pod_name, labs_pod_to_addr):
    """Map a Prometheus kubernetes_pod_name to a 64-char canonical peer_id, or None."""
    if pod_name.startswith("peer_id:"):
        rest = pod_name[len("peer_id:"):]
        if "//" in rest:
            rest = rest.split("//", 1)[1]
        if rest.startswith("0x") and re.fullmatch(r"0x[0-9a-fA-F]+", rest):
            return normalize64(rest)
    if pod_name in labs_pod_to_addr:
        return labs_pod_to_addr[pod_name]
    return None


def build_labs_pod_map():
    """Build mapping from labs pod name → peer_id, derived from the live cluster.
    The vmagent labels for labs validators look like
    `mainnet-validator-NAME-aptos-node-0-validator-0` and don't include the
    peer_id directly. We resolve them via well-known hostname patterns: each
    labs validator's network address is val0.NAME.mainnet.aptoslabs.com."""
    return {}  # populated dynamically below


def collect_metrics(token, win_avg, win_max, samples_step_avg, samples_step_max):
    """Run the avg_over_time and max_over_time queries against Grafana. Aggregates
    across (run_uuid, instance) by avg_by(kubernetes_pod_name) before applying
    the time-window aggregator — this dampens stale-instance contributions but
    keeps real signal from live instances.

    Computes per-validator FAILURE RATE (a fraction in [0,1]), not the raw
    `aptos_failed_proposals_in_window` gauge count. Earlier versions stored
    the gauge value directly which then got multiplied by 100 in the forge
    failpoint code as if it were a percentage — but the gauge is a COUNT of
    failed proposals in the LR window (with default mainnet config that's a
    ~1070-block window with ~10 leader rounds per validator), not a fraction.
    Treating it as a percentage over-modeled chronic failure by ~10x.

    Right metric is failed / (failed + committed) per validator.
    `clamp_min(_, 1)` guards against divide-by-zero for validators that have
    not led a round in the window.
    """
    rate_expr = (
        "(aptos_failed_proposals_in_window{chain_name=\"mainnet\"} / "
        "clamp_min(aptos_failed_proposals_in_window{chain_name=\"mainnet\"} + "
        "aptos_committed_proposals_in_window{chain_name=\"mainnet\"}, 1))"
    )
    fp_7d_q = (
        f"avg by (kubernetes_pod_name) ("
        f"  avg_over_time({rate_expr}[{win_avg}])"
        f")"
    )
    fp_30d_q = (
        f"max_over_time("
        f"  (avg by (kubernetes_pod_name) ({rate_expr}))"
        f"[{win_max}:{samples_step_max}])"
    )
    print(f"  query failure_rate_7d_avg over [{win_avg}]", file=sys.stderr)
    fp7 = grafana_query(fp_7d_q, token)
    print(f"  query failure_rate_30d_max over [{win_max}:{samples_step_max}]", file=sys.stderr)
    fp30 = grafana_query(fp_30d_q, token)
    return fp7, fp30


def aggregate_by_peer(results, labs_pod_to_addr):
    """Map (kubernetes_pod_name → value) entries to peer_id, taking the max
    across vmagent/telemetry sources for the same validator."""
    by_peer = {}
    unmapped = []
    for entry in results:
        pod = entry["metric"].get("kubernetes_pod_name", "")
        try:
            val = float(entry["value"][1])
        except (KeyError, ValueError):
            continue
        peer = extract_peer_id(pod, labs_pod_to_addr)
        if peer is None:
            unmapped.append(pod)
            continue
        by_peer[peer] = max(val, by_peer.get(peer, 0.0))
    return by_peer, unmapped


def build_labs_addr_lookup(token):
    """Discover labs validators from the cluster: query a label-set that
    includes both kubernetes_pod_name AND peer_id, then read the mapping."""
    # The aptos_consensus_last_committed_round metric is reported by both vmagent
    # (with mainnet-validator-NAME pod label) AND telemetry-service (with
    # peer_id:aptos-NAME//0xHEX label). We can't directly cross-reference, but
    # we can rely on the convention that vmagent labs pod names embed NAME
    # which matches the corresponding telemetry-service peer_id:aptos-NAME prefix.
    res = grafana_query(
        'group by (kubernetes_pod_name) (aptos_consensus_last_committed_round{chain_name="mainnet"})',
        token,
    )
    name_to_peer = {}  # NAME (e.g. "apne1-0") -> 64-char peer_id
    pod_to_name = {}  # vmagent pod -> NAME
    for entry in res:
        pod = entry["metric"].get("kubernetes_pod_name", "")
        m = re.match(r"peer_id:aptos-([\w\-]+?)//0x([0-9a-fA-F]+)", pod)
        if m:
            name_to_peer[m.group(1)] = "0x" + m.group(2).lower().zfill(64)
            continue
        m = re.match(r"mainnet-validator-([\w\-]+?)-aptos-node-0-validator-0", pod)
        if m:
            pod_to_name[pod] = m.group(1)
    labs_pod_to_addr = {}
    for pod, name in pod_to_name.items():
        peer = name_to_peer.get(name)
        if peer:
            labs_pod_to_addr[pod] = peer
    return labs_pod_to_addr


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--token", default=os.environ.get("GRAFANA_TOKEN"),
                    help="Grafana service-account token (env GRAFANA_TOKEN)")
    repo_root = Path(__file__).resolve().parent.parent
    default_out = repo_root / "testsuite/testcases/src/data/mainnet-mirror/failure_metrics.json"
    ap.add_argument("--out", default=str(default_out), help="Output failure_metrics.json path")
    ap.add_argument("--window-7d", default="7d", help="Window for fp_7d_avg (default: 7d)")
    ap.add_argument("--window-30d", default="30d", help="Window for fp_30d_max (default: 30d)")
    ap.add_argument("--step-30d", default="30m",
                    help="Subquery step for fp_30d_max sampling (default: 30m). Larger steps are faster "
                         "but may miss short-lived spikes.")
    args = ap.parse_args()

    if not args.token:
        print("error: GRAFANA_TOKEN env var or --token required", file=sys.stderr)
        sys.exit(1)

    print("Discovering active validator set ...", file=sys.stderr)
    active_addrs = fetch_active_validators()
    print(f"  {len(active_addrs)} active validators", file=sys.stderr)

    canonical_to_native = {normalize64(a): a for a in active_addrs}

    print("Resolving labs pod → peer_id mapping ...", file=sys.stderr)
    labs_pod_to_addr = build_labs_addr_lookup(args.token)
    print(f"  {len(labs_pod_to_addr)} labs pods mapped", file=sys.stderr)

    print("Querying failure metrics ...", file=sys.stderr)
    fp7, fp30 = collect_metrics(
        token=args.token,
        win_avg=args.window_7d,
        win_max=args.window_30d,
        samples_step_avg="5m",
        samples_step_max=args.step_30d,
    )

    fp7_by_peer, fp7_unmapped = aggregate_by_peer(fp7, labs_pod_to_addr)
    fp30_by_peer, fp30_unmapped = aggregate_by_peer(fp30, labs_pod_to_addr)
    if fp7_unmapped:
        print(f"  warning: {len(fp7_unmapped)} fp_7d series unmapped:", file=sys.stderr)
        for u in fp7_unmapped[:5]:
            print(f"    {u}", file=sys.stderr)

    metrics = {}
    for canon, native in canonical_to_native.items():
        v7 = fp7_by_peer.get(canon)
        v30 = fp30_by_peer.get(canon)
        if (v7 or 0) == 0 and (v30 or 0) == 0:
            continue
        metrics[native] = {
            "fp_7d_avg": round(v7, 4) if v7 is not None else None,
            "fp_30d_max": round(v30, 4) if v30 is not None else None,
        }

    out = {
        "_comment": (
            "Per-validator FAILURE RATE (fractions in [0,1]) — fraction of this validator's "
            "leader rounds that ended in failed_proposer_indices. "
            "Computed as failed/(failed+committed) over the LR rolling window, then averaged "
            "(7d) or max-aggregated (30d) over time. "
            "Field semantics changed from earlier versions: previously fp_7d_avg was the raw "
            "aptos_failed_proposals_in_window gauge AVG (a count, not a fraction); the forge "
            "failpoint code multiplied that by 100 as if it were a fraction, which over-modeled "
            "chronic failure. Now both fields are true fractions: fp_7d_avg=0.025 means 2.5%."
        ),
        "_pulled_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "metrics": dict(sorted(metrics.items())),
    }
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w") as f:
        json.dump(out, f, indent=2)
        f.write("\n")
    print(f"\nWrote {len(metrics)} entries to {out_path}", file=sys.stderr)

    # Summary by classification
    def classify(m):
        avg = m.get("fp_7d_avg") or 0
        spike = m.get("fp_30d_max") or 0
        if avg >= 0.10:
            return "StableChronic"
        if avg >= 0.02:
            return "OnlineButFlaky"
        if spike >= 0.40:
            return "EpisodicSpike"
        return "Healthy"

    from collections import Counter
    classes = Counter(classify(m) for m in metrics.values())
    print(f"\nClassification (excluding {len(active_addrs) - len(metrics)} Healthy validators with no failures):", file=sys.stderr)
    for cls in ("StableChronic", "OnlineButFlaky", "EpisodicSpike"):
        items = sorted(
            [(a, m) for a, m in metrics.items() if classify(m) == cls],
            key=lambda x: -(x[1].get("fp_7d_avg") or 0) - (x[1].get("fp_30d_max") or 0) / 100,
        )
        print(f"  {cls} ({len(items)}):", file=sys.stderr)
        for addr, m in items:
            print(
                f"    {addr[:14]:14} fp_7d={(m.get('fp_7d_avg') or 0):.3f} "
                f"fp_30d_max={(m.get('fp_30d_max') or 0):.3f}",
                file=sys.stderr,
            )


if __name__ == "__main__":
    main()
