#!/usr/bin/env python3
"""Leaf-class sum, CPU-weighted on the largest txn_executor thread.

How much of the thread is allocator / hashing / memcpy / clocks rather than
work the transaction actually asked for.

Usage: plumb.py <prof> <syms>
"""
import os
import sys, collections, gzip, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3

NTXN = int(os.environ.get("NTXN", "14030"))
CLASSES = [
    ("clock", r"mach_absolute_time|mach_timebase_info|clock_gettime|Instant::now"),
    ("hash", r"hashbrown|make_hash|ahash|AHasher|RandomState|sip::Hasher|DefaultHasher"
             r"|StructTag::hash|IndexMap::hash|DashMap::hash"),
    ("keccak", r"keccak|sha3|Sha3|CryptoHash"),
    ("alloc", r"_rjem_|^malloc|^free$|sdallocx|rallocx|extent_|raw_vec|RawVecInner|dealloc"
              r"|arena_ralloc|arena_cache_bin|Allocator"),
    ("memops", r"_platform_mem|_platform_bzero|OUTLINED_FUNCTION"),
    ("dropglue", r"drop_glue"),
    ("btree", r"btree::|StateKeyInner::cmp"),
    ("tls", r"pthread_getspecific|TLS wrapper|LocalKey"),
]

prof, syms = sys.argv[1], sys.argv[2]
p = json.load(gzip.open(prof, "rt"))
libs = an3.load_syms(syms)
lib_dbgid = [l["breakpadId"][:32].upper() for l in p["libs"]]
cands = [t for t in p["threads"] if t.get("name") == "txn_executor"]
th = max(cands, key=lambda t: t["samples"].get("length", len(t["samples"]["stack"])))
sa = th["stringArray"]
ft, fr, st, s = th["funcTable"], th["frameTable"], th["stackTable"], th["samples"]
res_lib, fn_res, fn_name = th["resourceTable"]["lib"], ft["resource"], ft["name"]
fr_func, fr_addr, sf = fr["func"], fr["address"], st["frame"]
cache = {}


def frame_sym(fi):
    r = cache.get(fi)
    if r is None:
        fu = fr_func[fi]
        nm = sa[fn_name[fu]]
        if nm.startswith("0x"):
            ri = fn_res[fu]
            dbg = None
            if ri is not None and ri >= 0:
                li = res_lib[ri]
                if li is not None and li >= 0:
                    dbg = lib_dbgid[li]
            nm = (an3.sym_for(libs, dbg, fr_addr[fi]) if dbg else None) or nm
        r = an3.norm(nm)
        cache[fi] = r
    return r


cls_cache = {}


def classify(nm):
    r = cls_cache.get(nm)
    if r is None:
        r = ""
        for name, rx in CLASSES:
            if re.search(rx, nm):
                r = name
                break
        cls_cache[nm] = r
    return r


n = s.get("length", len(s["stack"]))
acc = collections.Counter()
tot = 0.0
for i in range(n):
    si = s["stack"][i]
    w = s["threadCPUDelta"][i] or 0.0
    tot += w
    if si is None:
        continue
    c = classify(frame_sym(sf[si]))
    if c:
        acc[c] += w

print(f"thread total {tot/NTXN:.2f} us/txn")
run = 0.0
for name, _ in CLASSES:
    v = acc[name] / NTXN
    run += v
    print(f"  {name:9s} {v:6.2f} us/txn  {100*v*NTXN/tot:5.2f}%")
print(f"  {'TOTAL':9s} {run:6.2f} us/txn  {100*run*NTXN/tot:5.2f}%")
