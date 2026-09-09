#!/usr/bin/env python3
"""Exclusive partition of the executor thread by leaf symbol class.

Usage: part.py <prof> <syms>

Every sample lands in exactly one bucket, chosen by the first matching rule in
order, so the buckets sum to the thread total.
"""
import os
import sys, collections, gzip, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import an3
NTXN = int(os.environ.get("NTXN", "14030"))
prof, syms = sys.argv[1], sys.argv[2]

# (bucket, leaf regex, required-ancestor regex or None). First match wins.
RULES = [
    ("instrumentation", r"Timespec::now|inc_with_by|observe_with|mach_absolute_time|"
                        r"__commpage_gettimeofday", None),
    ("storage backend", r"rocksdb|LZ4_|aptos_db::|aptos_schemadb|layered_map|"
                        r"StateSlot|snappy|crc32|leveldb", None),
    ("hashing", r"sip::Hasher|DefaultHasher|RandomState|ahash|make_hash|"
                r"tiny_keccak|Sha3|sha3|fxhash", None),
    ("hash probing", r"hashbrown|dashmap|RawTable|hash_map|HashMap::", None),
    ("allocator", r"_rjem|malloc|free$|sdallocx|je_arena|je_tcache|je_bin|"
                  r"operator new|operator delete", None),
    ("memory traffic", r"_platform_memmove|_platform_memset|_platform_memcmp|"
                       r"_platform_bzero|memcpy", None),
    ("thread sync", r"psynch|pthread_|ulock|os_unfair|sem_|park|Condvar", None),
    ("interpreter", r"mono_move_runtime::interpreter", None),
    ("natives", r"mono_move_natives|native_context", None),
    ("loader", r"mono_move_loader|prepared_module|specializ|verif", None),
    ("global context", r"mono_move_global_context", None),
    ("mono storage", r"mono_move_runtime::global_storage|mono_move_runtime::heap|"
                     r"mono_move_runtime::session|resource_provider", None),
    ("materialize", r"mono_move_output|materialize|serialize_into|bcs::|"
                    r"write_set|WriteOp|to_contract_events", None),
    ("state key", r"state_key|StateKey|identifier::is_valid|Identifier|"
                  r"AccessPath|language_storage", None),
    ("block executor", r"aptos_block_executor|aptos_mvhashmap|aptos_vm|"
                       r"aptos_types|aptos_storage_interface", None),
    ("legacy move vm", r"move_vm_runtime|move_vm_types|move_binary_format|"
                       r"move_core_types", None),
]
COMPILED = [(b, re.compile(r), (re.compile(a) if a else None)) for b, r, a in RULES]

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


def bucket(si):
    r = bmemo.get(si)
    if r is None:
        leaf = frame_sym(sf[si])
        r = "unclassified"
        for name, rx, _ in COMPILED:
            if rx.search(leaf):
                r = name
                break
        bmemo[si] = r
    return r


n = s.get("length", len(s["stack"]))
c = collections.Counter()
unc = collections.Counter()
tot = 0.0
for i in range(n):
    si = s["stack"][i]
    w = s["threadCPUDelta"][i] or 0.0
    tot += w
    if si is None:
        c["idle/no-stack"] += w
        continue
    b = bucket(si)
    c[b] += w
    if b == "unclassified":
        unc[frame_sym(sf[si])] += w
print(f"thread total {tot/NTXN:.2f} us/txn\n")
for k, v in c.most_common():
    print(f"{v/NTXN:7.2f}us {100*v/tot:5.1f}%  {k}")
if unc:
    print("\n-- top unclassified --")
    for k, v in unc.most_common(15):
        print(f"{v/NTXN:7.3f}us  {k[:100]}")
