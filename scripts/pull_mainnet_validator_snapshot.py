#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Build the enriched mainnet validator snapshot used by the forge "mainnet shape"
test suite. For each active validator it captures: peer_id, voting_power, the
extracted IP / hostname from `network_addresses`, geolocation (country, city,
org), and a forge region bucket.

Two input modes:
  - default (recommended): load the already-pulled validator set from
    testsuite/testcases/src/data/mainnet-mirror/stake_distribution.json. Avoids
    re-hitting mainnet RPC on every iteration; geolocation is the slow /
    rate-limited step.
  - --from-rest-api: fetch 0x1::stake::ValidatorSet live (use after a long gap
    or to refresh the underlying validator set; will overwrite the input file
    when --refresh-input is also passed).

Output: testsuite/testcases/src/data/mainnet-mirror/mainnet_validator_snapshot.json
by default. The output file is `include_str!`-embedded into aptos-testcases at
compile time, so committing a refreshed snapshot is a normal code change.

Usage:
    python3 scripts/pull_mainnet_validator_snapshot.py
    IPINFO_TOKEN=... python3 scripts/pull_mainnet_validator_snapshot.py
    python3 scripts/pull_mainnet_validator_snapshot.py --from-rest-api --refresh-input
"""

import argparse
import json
import os
import re
import socket
import ssl
import sys
import time
import urllib.error
import urllib.request
from collections import defaultdict
from pathlib import Path


def _make_ssl_context():
    """Build an SSL context that works even on macOS python.org installs where
    the system CA bundle isn't wired up. Tries certifi, then /etc/ssl/cert.pem,
    then falls back to the default."""
    try:
        import certifi
        return ssl.create_default_context(cafile=certifi.where())
    except ImportError:
        pass
    if os.path.exists("/etc/ssl/cert.pem"):
        return ssl.create_default_context(cafile="/etc/ssl/cert.pem")
    return ssl.create_default_context()


_SSL_CTX = _make_ssl_context()

REST_API_DEFAULT = "https://fullnode.mainnet.aptoslabs.com/v1"
IPINFO_BASE = "https://ipinfo.io"

# Region buckets matching forge mainnet_calibrated_six_regions().
# Order matches the BTreeMap key order in MultiRegionNetworkEmulationConfig.
# Country mappings are coarse — they cover the common cases; off-grid validators
# fall through to "unknown" and the suite can either drop them or pin them to a
# default bucket.
REGION_BY_COUNTRY = {
    # apne1
    "JP": "apne1", "KR": "apne1", "TW": "apne1", "HK": "apne1", "SG": "apne1",
    # eu-central1 (Germany / central+eastern EU)
    "DE": "eu-central1", "AT": "eu-central1", "CH": "eu-central1",
    "PL": "eu-central1", "CZ": "eu-central1",
    # eu-west-1 (Ireland / western EU + Baltics)
    "IE": "eu-west-1", "FR": "eu-west-1", "NL": "eu-west-1", "BE": "eu-west-1",
    "GB": "eu-west-1", "ES": "eu-west-1", "PT": "eu-west-1", "IT": "eu-west-1",
    "SE": "eu-west-1", "FI": "eu-west-1", "NO": "eu-west-1", "DK": "eu-west-1",
    "LT": "eu-west-1", "LV": "eu-west-1", "EE": "eu-west-1",
    # sa-east-1
    "BR": "sa-east-1", "AR": "sa-east-1", "CL": "sa-east-1",
    # ca-central-1
    "CA": "ca-central-1",
    # us-central1 (covers all US + Mexico)
    "US": "us-central1", "MX": "us-central1",
    # apne1 (extended for east-asian community validators)
    "CN": "apne1", "TH": "apne1", "VN": "apne1", "ID": "apne1",
    # Russia tends to host in Moscow / European Russia → bucket with eu-central1
    "RU": "eu-central1",
}

# Hostname hints override geolocation for known labs naming patterns.
HOSTNAME_HINTS = [
    (re.compile(r"\bapne1-\d+\b"), "apne1"),
    (re.compile(r"\beuwe\d-\d+\b"), "eu-west-1"),
    (re.compile(r"\beuce\d-\d+\b"), "eu-central1"),
]

# Hostname fallback region used when geolocation fails (DNS unresolvable).
# Sourced from operator-side info; revisit when the operator's infra is back up
# or a new validator joins under one of these domains.
HOSTNAME_FALLBACK_REGION = [
    # latitude.sh-delegation (peer_id 0x61b64e39…) — known wedged on mainnet,
    # operator runs out of Frankfurt, DE. See memory latitude-validator-wedged-2026-04-17.md
    (re.compile(r"nodexpert\.xyz$"), "eu-central1"),
]


def http_get(url, timeout=15):
    req = urllib.request.Request(url, headers={"User-Agent": "aptos-forge-snapshot/1.0"})
    with urllib.request.urlopen(req, timeout=timeout, context=_SSL_CTX) as f:
        return json.load(f)


def fetch_validator_set(rest_api):
    """Fetch live validator set in the same shape as the existing
    mainnet-mirror-data/stake_distribution.json (a flat list, one entry per
    active validator, sorted by voting power)."""
    url = f"{rest_api}/accounts/0x1/resource/0x1::stake::ValidatorSet"
    data = http_get(url)
    actives = data["data"]["active_validators"]
    actives_sorted = sorted(actives, key=lambda v: int(v["voting_power"]), reverse=True)
    out = []
    for rank, v in enumerate(actives_sorted, start=1):
        addr = v["addr"]
        voting_power = int(v["voting_power"])
        out.append({
            "addr": addr,
            "addr_prefix8": addr[2:10] if addr.startswith("0x") else addr[:8],
            "voting_power": voting_power,
            "voting_power_apt": voting_power / 1e8,
            "config": v["config"],
            "rank": rank,
        })
    return out


def is_public_ipv4(octets):
    if len(octets) != 4:
        return False
    a, b = octets[0], octets[1]
    if a == 0 or a == 10 or a == 127 or a >= 224:
        return False
    if a == 169 and b == 254:
        return False
    if a == 172 and 16 <= b <= 31:
        return False
    if a == 192 and b == 168:
        return False
    return True


def _parse_leb128(data, off):
    res = 0
    shift = 0
    while True:
        if off >= len(data):
            raise ValueError("LEB128 ran past end of buffer")
        b = data[off]
        off += 1
        res |= (b & 0x7F) << shift
        if not (b & 0x80):
            return res, off
        shift += 7


def parse_network_addresses(hex_str):
    """Proper BCS decode of `Vec<NetworkAddress>`.

    Each NetworkAddress is `LEB128 byte_length`, then a blob containing
    `LEB128 num_protocols` followed by a tagged sequence:
      0x00 Ip4  (4 bytes)
      0x01 Ip6  (16 bytes)
      0x02 Dns  (LEB128 length + ASCII bytes)
      0x03 Dns4 (LEB128 length + ASCII bytes)
      0x04 Dns6 (LEB128 length + ASCII bytes)
      0x05 Tcp  (u16 little-endian port, 2 bytes)
      0x06 Memory (LEB128 port)
      0x07 NoiseIK (LEB128 length + raw bytes — must be skipped over precisely
                   or its bytes can be misread as a fake IPv4)
      0x08 Handshake (u8 version, 1 byte)

    Returns (first_public_ip, first_dns_hostname). The earlier heuristic decoder
    incorrectly scanned for `0x00 + 4 bytes` patterns globally, which produced
    spurious IPs from inside the 32-byte noise key (tag 0x07) — sending some
    validators to wrong continents.
    """
    if not hex_str:
        return None, None
    if hex_str.startswith("0x"):
        hex_str = hex_str[2:]
    try:
        raw = bytes.fromhex(hex_str)
    except ValueError:
        return None, None

    ip = None
    hostname = None
    try:
        n_addrs, pos = _parse_leb128(raw, 0)
        for _ in range(n_addrs):
            addr_len, pos = _parse_leb128(raw, pos)
            blob = raw[pos:pos + addr_len]
            pos += addr_len
            ap = 0
            n_proto, ap = _parse_leb128(blob, ap)
            for _ in range(n_proto):
                if ap >= len(blob):
                    break
                tag = blob[ap]
                ap += 1
                if tag == 0x00 and ap + 4 <= len(blob):
                    octets = list(blob[ap:ap + 4])
                    if ip is None and is_public_ipv4(octets):
                        ip = ".".join(str(b) for b in octets)
                    ap += 4
                elif tag == 0x01 and ap + 16 <= len(blob):
                    ap += 16  # Ip6 — skip; we only emit Ip4 + Dns
                elif tag in (0x02, 0x03, 0x04):
                    dlen, ap = _parse_leb128(blob, ap)
                    name = blob[ap:ap + dlen].decode("ascii", errors="replace")
                    ap += dlen
                    if hostname is None and re.fullmatch(r"[A-Za-z0-9._-]+\.[A-Za-z]{2,}", name):
                        hostname = name
                elif tag == 0x05 and ap + 2 <= len(blob):
                    ap += 2  # Tcp port (u16 LE)
                elif tag == 0x06:
                    _, ap = _parse_leb128(blob, ap)  # Memory port
                elif tag == 0x07:
                    klen, ap = _parse_leb128(blob, ap)
                    ap += klen  # NoiseIK key — skip
                elif tag == 0x08 and ap + 1 <= len(blob):
                    ap += 1  # Handshake version
                else:
                    break
    except (ValueError, IndexError):
        pass
    return ip, hostname


def resolve_hostname(hostname, retries=2):
    last_err = None
    for attempt in range(retries + 1):
        try:
            return socket.gethostbyname(hostname)
        except (socket.gaierror, socket.herror) as e:
            last_err = e
            if attempt < retries:
                time.sleep(0.5)
    return None


def geolocate(ip, token=None, retries=2):
    if not ip:
        return {}
    url = f"{IPINFO_BASE}/{ip}/json"
    if token:
        url += f"?token={token}"
    for attempt in range(retries + 1):
        try:
            return http_get(url, timeout=10)
        except urllib.error.HTTPError as e:
            if e.code == 429 and attempt < retries:
                time.sleep(2 ** attempt)
                continue
            return {"_error": f"HTTP {e.code}"}
        except Exception as e:
            return {"_error": str(e)}
    return {"_error": "geolocate failed"}


def bucket_region(country, hostname):
    if hostname:
        for pat, bucket in HOSTNAME_HINTS:
            if pat.search(hostname):
                return bucket
    if country and country in REGION_BY_COUNTRY:
        return REGION_BY_COUNTRY[country]
    # Last-resort: hostname pattern fallback (used when geolocation fails)
    if hostname:
        for pat, bucket in HOSTNAME_FALLBACK_REGION:
            if pat.search(hostname):
                return bucket
    return "unknown"


def network_addresses_hex(entry):
    """Pull network_addresses hex string from either flat or nested layouts."""
    flat = entry.get("network_addresses")
    if flat:
        return flat
    cfg = entry.get("config") or {}
    return cfg.get("network_addresses")


def classify_availability(metrics):
    """Map failure metrics to one of {Healthy, EpisodicSpike, OnlineButFlaky, StableChronic}.

    Thresholds derived from the 2026-04-30 mainnet failure pattern analysis
    (see memory mainnet-failure-pattern-2026-04-30.md). The 7d-avg is the
    primary signal; 30d-max is what surfaces episodic spike validators that
    look healthy in the 7d window but had a bad event earlier.

    Failure metrics input file is generated by scripts/pull_mainnet_failure_metrics.py;
    refresh that first if you want up-to-date classifications.
    """
    if metrics is None:
        return None
    avg = metrics.get("fp_7d_avg") or 0.0
    spike = metrics.get("fp_30d_max") or 0.0
    if avg >= 0.10:
        return "StableChronic"
    if avg >= 0.02:
        return "OnlineButFlaky"
    if spike >= 0.40:
        return "EpisodicSpike"
    return "Healthy"


def load_failure_metrics(path):
    """Load the per-peer_id failure metrics file. Returns {peer_id: metrics}
    or empty dict if the file doesn't exist."""
    if not path or not Path(path).exists():
        return {}
    with Path(path).open() as f:
        data = json.load(f)
    return data.get("metrics", {})


def enrich(entries, sleep_s, ipinfo_token, geolocate_enabled, failure_metrics):
    snapshot = []
    n = len(entries)
    for idx, e in enumerate(entries):
        peer_id = e["addr"]
        voting_power = int(e["voting_power"])
        net_hex = network_addresses_hex(e)

        ip, hostname = parse_network_addresses(net_hex)
        if not ip and hostname:
            ip = resolve_hostname(hostname)

        country = city = org = None
        if ip and geolocate_enabled:
            geo = geolocate(ip, token=ipinfo_token)
            country = geo.get("country")
            city = geo.get("city")
            org = geo.get("org")

        region = bucket_region(country, hostname)

        metrics = failure_metrics.get(peer_id)
        availability_class = classify_availability(metrics) or "Healthy"

        snapshot.append({
            "peer_id": peer_id,
            "rank": e.get("rank"),
            "voting_power": str(voting_power),
            "voting_power_apt": voting_power // 10**8,
            "ip": ip,
            "hostname": hostname,
            "country": country,
            "city": city,
            "org": org,
            "region": region,
            "availability_class": availability_class,
            "fp_7d_avg": metrics.get("fp_7d_avg") if metrics else None,
            "fp_30d_max": metrics.get("fp_30d_max") if metrics else None,
        })

        label = hostname or ip or "?"
        print(
            f"  [{idx + 1:>3}/{n}] {peer_id[:10]} "
            f"{region:13s} {country or '??':2s}  {availability_class:14s}  {label}",
            file=sys.stderr,
        )
        if geolocate_enabled and ip:
            time.sleep(sleep_s)
    return snapshot


def main():
    repo_root = Path(__file__).resolve().parent.parent
    snapshot_dir = repo_root / "testsuite/testcases/src/data/mainnet-mirror"
    default_input = snapshot_dir / "stake_distribution.json"
    default_out = snapshot_dir / "mainnet_validator_snapshot.json"
    default_failure_metrics = snapshot_dir / "failure_metrics.json"

    ap = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    ap.add_argument("--input", default=str(default_input),
                    help="Path to stake_distribution.json (default: mainnet-mirror-data/stake_distribution.json)")
    ap.add_argument("--out", default=str(default_out),
                    help="Output enriched snapshot path")
    ap.add_argument("--from-rest-api", action="store_true",
                    help="Fetch validator set live from mainnet REST API instead of reading --input")
    ap.add_argument("--rest-api", default=REST_API_DEFAULT,
                    help="Mainnet REST API base URL (used with --from-rest-api)")
    ap.add_argument("--refresh-input", action="store_true",
                    help="With --from-rest-api: also overwrite --input with the freshly fetched data")
    ap.add_argument("--sleep", type=float, default=0.15,
                    help="Sleep between geolocation requests")
    ap.add_argument("--ipinfo-token", default=os.environ.get("IPINFO_TOKEN"),
                    help="ipinfo.io token (env IPINFO_TOKEN also accepted)")
    ap.add_argument("--no-geolocate", action="store_true",
                    help="Skip ipinfo lookups (region bucketing falls back to hostname hints only)")
    ap.add_argument("--failure-metrics", default=str(default_failure_metrics),
                    help="Path to per-peer_id failure metrics JSON (used to assign availability_class)")
    args = ap.parse_args()

    if args.from_rest_api:
        print(f"Fetching ValidatorSet from {args.rest_api} ...", file=sys.stderr)
        entries = fetch_validator_set(args.rest_api)
        print(f"  {len(entries)} active validators", file=sys.stderr)
        if args.refresh_input:
            in_path = Path(args.input)
            in_path.parent.mkdir(parents=True, exist_ok=True)
            with in_path.open("w") as f:
                json.dump(entries, f, indent=2)
                f.write("\n")
            print(f"  refreshed {in_path}", file=sys.stderr)
    else:
        in_path = Path(args.input)
        if not in_path.exists():
            print(f"error: {in_path} does not exist; pass --from-rest-api to fetch live", file=sys.stderr)
            sys.exit(1)
        with in_path.open() as f:
            entries = json.load(f)
        print(f"Loaded {len(entries)} validators from {in_path}", file=sys.stderr)

    failure_metrics = load_failure_metrics(args.failure_metrics)
    if failure_metrics:
        print(f"Loaded failure metrics for {len(failure_metrics)} validators from {args.failure_metrics}",
              file=sys.stderr)

    snapshot = enrich(
        entries,
        sleep_s=args.sleep,
        ipinfo_token=args.ipinfo_token,
        geolocate_enabled=not args.no_geolocate,
        failure_metrics=failure_metrics,
    )

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source": ("mainnet REST API live" if args.from_rest_api else str(Path(args.input).name)),
        "validator_count": len(snapshot),
        "validators": snapshot,
    }
    with out.open("w") as f:
        json.dump(payload, f, indent=2)
        f.write("\n")
    print(f"\nWrote {len(snapshot)} entries to {out}", file=sys.stderr)

    counts = defaultdict(int)
    stake = defaultdict(int)
    total_stake = sum(int(e["voting_power"]) for e in snapshot)
    unresolved = sum(1 for e in snapshot if not e["ip"] and not e["hostname"])
    for e in snapshot:
        counts[e["region"]] += 1
        stake[e["region"]] += int(e["voting_power"])
    print("\nDistribution by region:", file=sys.stderr)
    print(f"  {'region':14s} {'count':>5s} {'count%':>7s} {'stake%':>7s}", file=sys.stderr)
    for r in sorted(counts):
        cnt = counts[r]
        cnt_pct = 100 * cnt / len(snapshot)
        stake_pct = 100 * stake[r] / total_stake if total_stake else 0
        print(f"  {r:14s} {cnt:>5d} {cnt_pct:>6.1f}% {stake_pct:>6.1f}%", file=sys.stderr)
    if unresolved:
        print(f"\n  {unresolved} validators had no extractable IP/hostname (region=unknown)", file=sys.stderr)

    if failure_metrics:
        avail_counts = defaultdict(int)
        for e in snapshot:
            avail_counts[e["availability_class"]] += 1
        print("\nDistribution by availability_class:", file=sys.stderr)
        for c in ("StableChronic", "OnlineButFlaky", "EpisodicSpike", "Healthy"):
            print(f"  {c:16s} {avail_counts[c]:>3d}", file=sys.stderr)


if __name__ == "__main__":
    main()
