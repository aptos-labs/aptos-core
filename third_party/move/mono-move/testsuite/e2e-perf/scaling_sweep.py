#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""EXPERIMENT ONLY. Thread-scaling sweep: V1 vs MonoMove on byte-identical
recorded blocks, at several execution thread counts.

Env:
  WORKLOADS   comma list
  THREADS     comma list
  REPEATS, BLOCKS, BLOCK_SIZE, ACCOUNTS
  PIN         if set, pin the benchmark to one hardware thread per core
  REPORT_PATH markdown report
  PROFILE     comma list of "vm:threads:workload" to record with --cpu-profiling
"""
import json, os, re, shutil, statistics, subprocess, sys, time
from collections import defaultdict

E = os.environ.get
WORKLOADS = E("WORKLOADS", "no-op,apt-fa-transfer").split(",")
THREADS = [int(t) for t in E("THREADS", "2,4,8,16").split(",")]
REPEATS = int(E("REPEATS", "3"))
BLOCKS = int(E("BLOCKS", "40"))
BLOCK_SIZE = int(E("BLOCK_SIZE", "1000"))
ACCOUNTS = int(E("ACCOUNTS", "500000"))
PIN = bool(E("PIN"))
REPORT_PATH = E("REPORT_PATH", "scaling_report.md")
VMS = E("VMS", "v1,mono").split(",")
PROFILE = [p.split(":") for p in E("PROFILE", "").split(",") if p]
BIN = "target/release/aptos-executor-benchmark"
ROOT = E("ROOT", "/tmp/scaling-sweep")
FLAG = "ENABLE_MONO_MOVE"


def sh(cmd, check=True):
    print(f"$ {cmd}", flush=True)
    p = subprocess.run(cmd, shell=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if check and p.returncode != 0:
        print(p.stdout[-20000:])
        raise SystemExit(f"FAILED ({p.returncode}): {cmd}")
    return p.stdout


def pin_prefix():
    if not PIN:
        return ""
    # One logical CPU per physical core.
    out = sh("lscpu -p=CPU,CORE,SOCKET", check=False)
    seen, cpus = set(), []
    for line in out.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        cpu, core, sock = line.split(",")[:3]
        if (core, sock) not in seen:
            seen.add((core, sock))
            cpus.append(cpu)
    return f"taskset -c {','.join(cpus)} "


def common(threads, db, cp):
    return (
        f"{pin_prefix()}{BIN} --block-executor-type aptos-vm-with-block-stm "
        f"--execution-threads {threads} --generate-then-execute --split-stages "
        f"--num-generator-workers 1 --block-size {BLOCK_SIZE} "
        f"run-executor --data-dir {db} --checkpoint-dir {cp}"
    )


NUM = r"(\d+\.?\d*)"


def comp(out, pat):
    m = re.findall(pat + r".*?\(component TPS: " + NUM + r"\)", out)
    return float(m[-1]) if m else None


def parse(out):
    return {
        "total": float(re.findall(r"Overall TPS: " + NUM, out)[-1]),
        "execution": comp(out, r"Overall fraction of total: \d+\.?\d* in execution"),
        "inner": comp(out, r"Overall fraction of execution \d+\.?\d* in inner block executor"),
        "concurrency": int(re.findall(r"Overall block executor concurrency: (\d+)", out)[-1]),
        "mono": FLAG in re.findall(r"Feature flag overrides after init: enable=\[([^\]]*)\]", out)[-1],
    }


def main():
    sh("nproc; lscpu | head -30; uname -a", check=False)
    print(sh("lscpu", check=False))
    print(sh("which perf; cat /proc/sys/kernel/perf_event_paranoid", check=False))
    sh(f"cargo build --release --package aptos-executor-benchmark")
    os.makedirs(ROOT, exist_ok=True)
    db = os.path.join(ROOT, "db")
    sh(
        f"{BIN} --block-executor-type aptos-vm-with-block-stm --block-size 1000 "
        f"--execution-threads 32 create-db --data-dir {db} --num-accounts {ACCOUNTS}"
    )
    rows = []
    for w in WORKLOADS:
        blocks = os.path.join(ROOT, f"{w}.blocks")
        rec = os.path.join(ROOT, f"{w}-rec")
        cp = os.path.join(ROOT, "cp")
        try:
            sh(
                f"{common(8, db, rec)} --transaction-type {w} --module-working-set-size 1 "
                f"--main-signer-accounts {2 * BLOCK_SIZE} "
                f"--additional-dst-pool-accounts {2 * BLOCK_SIZE * BLOCKS} "
                f"--blocks {BLOCKS} --dump-blocks {blocks}"
            )
        except SystemExit as e:
            print(f"record failed for {w}: {e}")
            continue
        for r in range(REPEATS):
            order = [(t, vm) for t in THREADS for vm in VMS]
            if r % 2 == 1:
                order.reverse()
            for t, vm in order:
                shutil.rmtree(cp, ignore_errors=True)
                ov = "--enable-feature-after-init" if vm == "mono" else "--disable-feature-after-init"
                t0 = time.time()
                try:
                    out = sh(f"{common(t, rec, cp)} --replay-blocks {blocks} {ov} {FLAG}")
                except SystemExit as e:
                    print(f"replay failed {w} {vm} {t}: {e}")
                    continue
                s = parse(out)
                row = dict(workload=w, vm=vm, threads=t, repeat=r, wall=round(time.time() - t0, 1),
                           load=os.getloadavg()[0], **s)
                print("SCALING_RESULT " + json.dumps(row), flush=True)
                rows.append(row)
        for vm, t, pw in PROFILE:
            if pw != w:
                continue
            shutil.rmtree(cp, ignore_errors=True)
            ov = "--enable-feature-after-init" if vm == "mono" else "--disable-feature-after-init"
            os.makedirs(os.path.join(ROOT, "prof"), exist_ok=True)
            name = os.path.join(ROOT, "prof", f"{vm}-{t}-{w}")
            cmd = f"{common(int(t), rec, cp)} --replay-blocks {blocks} {ov} {FLAG}"
            if shutil.which("samply"):
                out = sh(f"samply record --save-only --unstable-presymbolicate -o {name}.json.gz -- {cmd}", check=False)
            else:
                shutil.rmtree("profiling_results", ignore_errors=True)
                out = sh(cmd.replace(" --block-executor-type", " --cpu-profiling --block-executor-type"), check=False)
                if os.path.exists("profiling_results/cpu_flamegraph.svg"):
                    shutil.copy("profiling_results/cpu_flamegraph.svg", f"{name}.svg")
            print("PROFILE_RUN", vm, t, w, parse(out) if "Overall TPS" in out else out[-3000:])
        shutil.rmtree(rec, ignore_errors=True)
        shutil.rmtree(cp, ignore_errors=True)

    g = defaultdict(list)
    for r in rows:
        g[(r["workload"], r["vm"], r["threads"])].append(r)
    lines = [f"### Scaling sweep (block size {BLOCK_SIZE}, {BLOCKS} blocks, {REPEATS} repeats, pin={PIN})", ""]
    for metric in ["inner", "execution"]:
        lines += [f"#### {metric} TPS (median, spread, x vs lowest thread count)", ""]
        lines += ["| workload | vm | " + " | ".join(f"{t}T" for t in THREADS) + " |",
                  "|---|---|" + "---|" * len(THREADS)]
        for w in WORKLOADS:
            for vm in VMS:
                cells, base = [], None
                for t in THREADS:
                    rs = g.get((w, vm, t))
                    if not rs:
                        cells.append("-")
                        continue
                    vals = [x[metric] for x in rs]
                    med = statistics.median(vals)
                    spread = (max(vals) - min(vals)) / med * 100
                    base = base or med
                    cells.append(f"{med:.0f} ±{spread:.1f}% x{med / base:.2f}")
                lines.append(f"| {w} | {vm} | " + " | ".join(cells) + " |")
        lines.append("")
    report = "\n".join(lines)
    print(report)
    with open(REPORT_PATH, "w") as f:
        f.write(report)


if __name__ == "__main__":
    main()
