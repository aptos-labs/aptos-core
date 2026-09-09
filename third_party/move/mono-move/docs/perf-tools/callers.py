#!/usr/bin/env python3
"""Top ancestor chains for samples whose leaf matches a pattern."""
import os, sys, collections, gzip, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3

prof, syms, thread, pat = sys.argv[1:5]
depth = int(sys.argv[5]) if len(sys.argv) > 5 else 6

p = json.load(gzip.open(prof, "rt"))
libs = an3.load_syms(syms)
lib_dbgid = [l["breakpadId"][:32].upper() for l in p["libs"]]
chains = collections.Counter()
total = 0
for th in p["threads"]:
    if th.get("name") != thread:
        continue
    sa = th["stringArray"]
    ft, fr, st, samples = th["funcTable"], th["frameTable"], th["stackTable"], th["samples"]
    res_lib, fn_res, fn_name = th["resourceTable"]["lib"], ft["resource"], ft["name"]
    fr_func, fr_addr, sp, sf = fr["func"], fr["address"], st["prefix"], st["frame"]
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
    n = samples.get("length", len(samples["stack"]))
    for i in range(n):
        s = samples["stack"][i]
        if s is None:
            continue
        total += 1
        if pat not in frame_sym(sf[s]):
            continue
        chain = []
        cur = sp[s]
        while cur is not None and len(chain) < depth:
            chain.append(frame_sym(sf[cur]))
            cur = sp[cur]
        chains[tuple(chain)] += 1
print(f"total={total} matching={sum(chains.values())}")
for c, v in chains.most_common(12):
    print(f"{v:5d}  " + " <- ".join(x[:60] for x in c))
