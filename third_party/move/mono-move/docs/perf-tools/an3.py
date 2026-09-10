#!/usr/bin/env python3
"""Aggregate a samply profile (+ .syms.json) for one thread.

Symbols are normalized to the callee path with generic arguments stripped, so
substring matching cannot be fooled by type parameters embedded in a
monomorphized name.
"""
import gzip, json, sys, collections, bisect, re


def load_syms(path):
    d = json.load(open(path))
    st = d["string_table"]
    libs = {}
    for lib in d["data"]:
        tab = lib["symbol_table"]
        libs[lib["debug_id"].replace("-", "").upper()] = (
            [e["rva"] for e in tab],
            [st[e["symbol"]] for e in tab],
        )
    return libs


def sym_for(libs, debug_id, addr):
    ent = libs.get(debug_id)
    if ent is None or addr is None:
        return None
    rvas, names = ent
    i = bisect.bisect_right(rvas, addr) - 1
    return names[i] if i >= 0 else None


def strip_generics(s):
    out, depth = [], 0
    for ch in s:
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(ch)
    return "".join(out)


def norm(name):
    """`<Foo<A> as Bar>::baz::<C>` -> `Foo::baz`; `a::b::<C>` -> `a::b`."""
    if name.startswith("<"):
        depth = 0
        for i, ch in enumerate(name):
            if ch == "<":
                depth += 1
            elif ch == ">":
                depth -= 1
                if depth == 0:
                    inner, rest = name[1:i], name[i + 1:]
                    break
        else:
            return strip_generics(name)
        if " as " in inner:
            inner = inner.split(" as ")[0]
        return strip_generics(inner) + strip_generics(rest)
    return strip_generics(name)


def collect(profile_path, syms_path, thread_name):
    p = json.load(gzip.open(profile_path, "rt"))
    libs = load_syms(syms_path)
    lib_dbgid = [l["breakpadId"][:32].upper() for l in p["libs"]]

    self_c = collections.Counter()
    stacks = []  # list of (leaf_norm, tuple_of_norm_ancestors)
    total = 0
    for th in p["threads"]:
        if th.get("name") != thread_name:
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
                    nm = (sym_for(libs, dbg, fr_addr[fi]) if dbg else None) or nm
                r = norm(nm)
                cache[fi] = r
            return r

        # Memoize the normalized ancestor set per stack node.
        anc = {}

        def ancestors(s):
            r = anc.get(s)
            if r is None:
                pref = sp[s]
                base = ancestors(pref) if pref is not None else frozenset()
                r = base | {frame_sym(sf[s])}
                anc[s] = r
            return r

        n = samples.get("length", len(samples["stack"]))
        for i in range(n):
            s = samples["stack"][i]
            if s is None:
                continue
            total += 1
            leaf = frame_sym(sf[s])
            self_c[leaf] += 1
            stacks.append((leaf, ancestors(s)))
    return total, self_c, stacks


CRATE = re.compile(r"^&?(?:mut )?([A-Za-z_][A-Za-z0-9_]*)")

if __name__ == "__main__":
    prof, syms, thread = sys.argv[1], sys.argv[2], sys.argv[3]
    focus = sys.argv[4].split("|") if len(sys.argv) > 4 else []
    total, self_c, stacks = collect(prof, syms, thread)
    print(f"== {prof} thread={thread} samples={total}")
    by_crate = collections.Counter()
    for k, v in self_c.items():
        m = CRATE.match(k)
        by_crate[m.group(1) if m else "?"] += v
    print("\n-- self by crate (top 22) --")
    for k, v in by_crate.most_common(22):
        print(f"{100*v/total:7.2f}%  {v:6d}  {k}")
    print("\n-- self by symbol (top 35) --")
    for k, v in self_c.most_common(35):
        print(f"{100*v/total:7.2f}%  {v:6d}  {k[:120]}")
    if focus:
        print("\n-- inclusive --")
        inc = collections.Counter()
        for _leaf, a in stacks:
            for f in focus:
                if any(f in x for x in a):
                    inc[f] += 1
        for f in focus:
            print(f"{100*inc[f]/total:7.2f}%  {inc[f]:6d}  {f}")
