#!/usr/bin/env python3
"""CPU-weighted inclusive cost of named subtrees on the largest txn_executor
thread. Regions may overlap; each is reported independently."""
import os
import sys, collections, gzip, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3

NTXN = int(os.environ.get("NTXN", "14030"))
prof, syms = sys.argv[1:3]
rxs = [(n, re.compile(p)) for n, p in (a.split("=", 1) for a in sys.argv[3:])]

p = json.load(gzip.open(prof, "rt"))
libs = an3.load_syms(syms)
lib_dbgid = [l["breakpadId"][:32].upper() for l in p["libs"]]
cands = [t for t in p["threads"] if t.get("name") == "txn_executor"]
th = max(cands, key=lambda t: t["samples"].get("length", len(t["samples"]["stack"])))
sa = th["stringArray"]
ft, fr, st, s = th["funcTable"], th["frameTable"], th["stackTable"], th["samples"]
res_lib, fn_res, fn_name = th["resourceTable"]["lib"], ft["resource"], ft["name"]
fr_func, fr_addr, sp, sf = fr["func"], fr["address"], st["prefix"], st["frame"]
cache = {}
def frame_sym(fi):
    r = cache.get(fi)
    if r is None:
        fu = fr_func[fi]; nm = sa[fn_name[fu]]
        if nm.startswith("0x"):
            ri = fn_res[fu]; dbg = None
            if ri is not None and ri >= 0:
                li = res_lib[ri]
                if li is not None and li >= 0: dbg = lib_dbgid[li]
            nm = (an3.sym_for(libs, dbg, fr_addr[fi]) if dbg else None) or nm
        r = an3.norm(nm); cache[fi] = r
    return r

# Bitmask of which regions a stack node is inside, memoized down the tree.
memo = {}
def mask(si):
    if si is None:
        return 0
    m = memo.get(si)
    if m is None:
        nm = frame_sym(sf[si])
        m = mask(sp[si])
        for i, (_, rx) in enumerate(rxs):
            if not (m >> i) & 1 and rx.search(nm):
                m |= 1 << i
        memo[si] = m
    return m

hit = collections.Counter()
tot = 0.0
n = s.get("length", len(s["stack"]))
for i in range(n):
    w = s["threadCPUDelta"][i] or 0.0
    tot += w
    m = mask(s["stack"][i])
    for j, (nm, _) in enumerate(rxs):
        if (m >> j) & 1:
            hit[nm] += w

print(f"thread total {tot/NTXN:.2f} us/txn over {NTXN} txns\n")
for nm, _ in rxs:
    print(f"  {nm:<28} {hit[nm]/NTXN:>7.2f} us/txn  {100*hit[nm]/tot:>5.2f}%")
