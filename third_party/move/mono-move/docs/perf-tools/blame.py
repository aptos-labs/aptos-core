#!/usr/bin/env python3
"""Blame a leaf class on its nearest non-matching ancestor.

Usage: blame.py <prof> <syms> <leaf-regex> [limit]

For every sample whose leaf symbol matches LEAF, walk up until a frame does not
match LEAF and does not match the generic-glue filter, then charge the sample to
that frame. Answers "who is doing all this malloc / memcpy".
"""
import os
import sys, collections, gzip, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3
NTXN = int(os.environ.get("NTXN", "14030"))
prof, syms, leaf_rx = sys.argv[1], sys.argv[2], re.compile(sys.argv[3])
lim = int(sys.argv[4]) if len(sys.argv) > 4 else 25
# Frames that are pure plumbing: keep walking past them.
GLUE = re.compile(r"^(alloc::|core::ptr::drop_glue|core::|std::|<alloc|hashbrown::raw::RawTable::(reserve|resize|alloc)|_rjem|malloc|free$|sdallocx|_platform_)")
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


bmemo = {}


def blame(si):
    r = bmemo.get(si)
    if r is None:
        cur = si
        r = "<root>"
        while cur is not None:
            nm = frame_sym(sf[cur])
            if not leaf_rx.search(nm) and not GLUE.search(nm):
                r = nm
                break
            cur = sp[cur]
        bmemo[si] = r
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
    if not leaf_rx.search(frame_sym(sf[si])):
        continue
    sel += w
    c[blame(si)] += w
print(f"thread total {tot/NTXN:.2f} us/txn; matching leaves {sel/NTXN:.2f} us/txn "
      f"({100*sel/tot:.1f}%)\n")
for k, v in c.most_common(lim):
    print(f"{v/NTXN:7.3f}us {100*v/sel:5.2f}%  {k[:110]}")
