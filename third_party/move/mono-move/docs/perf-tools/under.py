#!/usr/bin/env python3
"""Self-cost ranking restricted to samples whose stack passes through ANCESTOR.

Usage: under.py <prof> <syms> <ancestor-regex> [limit] [--exclude=<regex>]

The exclude regex, when given, drops samples that also pass through it below
the ancestor, so nested subsystems can be carved out.
"""
import os
import sys, collections, gzip, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3
NTXN = int(os.environ.get("NTXN", "14030"))
prof, syms, anc_rx = sys.argv[1], sys.argv[2], re.compile(sys.argv[3])
lim = 30
exc = None
for a in sys.argv[4:]:
    if a.startswith("--exclude="):
        exc = re.compile(a.split("=", 1)[1])
    else:
        lim = int(a)
p = json.load(gzip.open(prof, "rt"))
libs = an3.load_syms(syms)
lib_dbgid = [l["breakpadId"][:32].upper() for l in p["libs"]]
cands = [t for t in p["threads"] if t.get("name") == "txn_executor"]
th = max(cands, key=lambda t: t["samples"].get("length", len(t["samples"]["stack"])))
sa = th["stringArray"]
ft, fr, st, s = th["funcTable"], th["frameTable"], th["stackTable"], th["samples"]
res_lib, fn_res, fn_name = th["resourceTable"]["lib"], ft["resource"], ft["name"]
fr_func, fr_addr, sf, sp = fr["func"], fr["address"], st["frame"], st["prefix"]
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


# Memoized per stack node: (passes ancestor, passes exclude).
memo = {}


def flags(si):
    r = memo.get(si)
    if r is None:
        nm = frame_sym(sf[si])
        pi = sp[si]
        pa, pe = flags(pi) if pi is not None else (False, False)
        r = (pa or bool(anc_rx.search(nm)), pe or (exc is not None and bool(exc.search(nm))))
        memo[si] = r
    return r


n = s.get("length", len(s["stack"]))
c = collections.Counter()
tot = sel = 0.0
for i in range(n):
    si = s["stack"][i]
    w = s["threadCPUDelta"][i] or 0.0
    tot += w
    if si is None:
        continue
    a, e = flags(si)
    if not a or e:
        continue
    sel += w
    c[frame_sym(sf[si])] += w
print(f"thread total {tot/NTXN:.2f} us/txn; under ancestor {sel/NTXN:.2f} us/txn "
      f"({100*sel/tot:.1f}%)\n")
for k, v in c.most_common(lim):
    print(f"{v/NTXN:7.3f}us {100*v/sel:5.2f}%  {k[:110]}")
