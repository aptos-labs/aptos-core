#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Attributes the folded `perf` stacks a scaling sweep writes to components.

Each folded file holds `comm;outermost;...;leaf count` lines restricted to the
execution stage. The script reports, per file, self time by component and
inclusive time under a set of interesting frames, both as a share of worker
samples and as microseconds per transaction when `--txns` and the sample
frequency are known.

    analyze_profile.py --txns 100000 --freq 499 \\
      profiles/no-op-v1-2t.folded.gz profiles/no-op-v1-16t.folded.gz
"""

import argparse
import gzip
import re
from collections import defaultdict

from tabulate import tabulate

# First match wins, so the specific rules come first.
CATEGORIES = [
    ("kernel futex", r"^(futex_|do_futex|__x64_sys_futex|__futex)"),
    ("kernel page fault", r"(page_fault|handle_mm_fault|do_anonymous_page|clear_page|__alloc_pages|zap_pte|unmap_|do_madvise|__do_fault|alloc_pages|rmqueue|free_pcppages|lru_add|folio)"),
    ("kernel sched", r"^(schedule|__schedule|try_to_wake_up|wake_up_q|pick_next_task|finish_task_switch|dequeue_task|enqueue_task|__wake_up|ttwu_)"),
    ("kernel other", r"^(entry_SYSCALL|do_syscall|syscall_|__x64_sys|native_|_raw_spin|asm_|irq|__irq|sysvec|x86_|exc_page|__sys_|ksys_|vfs_|ext4_|__fget|__GI___|do_sys|kmem_cache|__kmalloc|psi_|update_curr|__update_load|native_write_msr|apic_|hrtimer|tick_|ktime|read_tsc|__switch_to|copy_user|__get_user|__put_user|security_|__cond_resched|rcu_|smp_call|flush_tlb|__x86_indirect|__fdget|__check_object|memcg|mem_cgroup|__mod_|__count_|__rcu|__local_bh)"),
    ("mono interpreter", r"^mono_move_runtime::interpreter"),
    ("mono heap/arena", r"^mono_move_runtime::(heap|arena|gc|session_heap)|SegmentedArena|FrozenHeap|evacuat"),
    ("mono serde/layout", r"^mono_move_runtime::(serde|serialize|deserialize|layout|codec)|mono_move_core::(layout|type_layout)"),
    ("mono runtime other", r"^mono_move_runtime"),
    ("mono loader", r"^mono_move_loader|^mono_move_core::loader|mono_move_core::module"),
    ("mono block-stm glue", r"^aptos_block_executor::mono_move|MonoTransactionExecutor|MonoTxnOutput|MonoReads|BlockSTMParallelProvider|ParallelReader"),
    ("mono txn executor", r"^mono_move_aptos_transaction_executor|^mono_move_aptos_state_view_providers"),
    ("mono core/context", r"^mono_move_core|^mono_move_global_context|^mono_move_"),
    ("legacy interpreter", r"^move_vm_runtime::interpreter"),
    ("legacy vm", r"^move_vm_runtime|^move_vm_types|^move_core_types|^move_binary_format|^move_bytecode_verifier"),
    ("mvhashmap", r"^aptos_mvhashmap"),
    ("block-stm scheduler", r"^aptos_block_executor::scheduler|SchedulerV2|AbortManager|ExecutionQueueManager"),
    ("block-stm view/reads", r"^aptos_block_executor::(view|captured_reads|cold_validation)|LatestView|CapturedReads"),
    ("block-stm executor", r"^aptos_block_executor"),
    ("siphash", r"sip::|SipHasher|siphash|core::hash::|DefaultHasher|RandomState"),
    ("other hashers", r"fxhash|rustc_hash|ahash|FxHasher|twox|xxhash|seahash"),
    ("dashmap", r"^dashmap"),
    ("hashbrown", r"^hashbrown|RawTable"),
    ("btree", r"alloc::collections::btree|BTreeMap|BTreeSet"),
    ("allocator", r"^(_rjem_|je_|rjem_|malloc|calloc|realloc|free$|sdallocx|mallocx|arena_|tcache_|extent_|large_|pages_|__rust_alloc|__rust_dealloc|alloc::alloc)|jemalloc|tikv_jemalloc|__rdl_|__libc_malloc|__libc_free"),
    ("memcpy/memset", r"^(__memmove|__memcpy|__memset|memcpy|memmove|memset|memcmp|bcmp|__memcmp|__strlen|copy_nonoverlapping)"),
    ("sha3/keccak", r"sha3|keccak|Keccak|aptos_crypto::hash"),
    ("crypto other", r"ed25519|curve25519|secp256k1|bls12|blst|aptos_crypto"),
    ("bcs/serde", r"^bcs|^serde|_serde|Serialize|Deserialize"),
    ("aptos-vm", r"^aptos_vm::|^aptos_vm_types|^aptos_vm_environment|^aptos_vm_logging|^aptos_gas|^aptos_native|^aptos_framework|^aptos_table_natives|^aptos_aggregator"),
    ("aptos types", r"^aptos_types|^aptos_crypto"),
    ("state view/store", r"^aptos_storage_interface|^aptos_scratchpad|^aptos_executor|^aptos_executor_types|^aptos_db|^layered_map|^aptos_experimental_layered|^aptos_jellyfish|^aptos_schemadb|CachedStateView|StateView|HotState|aptos_executor_benchmark"),
    ("rocksdb", r"rocksdb|^librocksdb|snappy|lz4|zstd"),
    ("sync (parking_lot/mutex)", r"^parking_lot|^lock_api|std::sync::|aptos_infallible|^crossbeam|Mutex|RwLock|Condvar|spin_loop|Backoff"),
    ("metrics", r"^prometheus|^aptos_metrics_core|Histogram|IntCounter|IntGauge"),
    ("rayon", r"^rayon"),
    ("std/core other", r"^(std::|core::|alloc::)"),
    ("libc other", r"^(__|_dl_|pthread|clock_gettime|__vdso|__clock|__GI)"),
    ("unknown", r"\[unknown\]"),
]
COMPILED = [(name, re.compile(pattern)) for name, pattern in CATEGORIES]

# Inclusive buckets: the first frame of a stack matching the pattern claims the
# sample. A stack can land in several buckets.
INCLUSIVE = [
    ("worker loop", r"worker_loop_v2"),
    ("execute_v2", r"::execute_v2"),
    ("  VM execute (mono)", r"MonoTransactionExecutor.*::execute\b|MonoTransactionExecutor as .*SingleTransactionExecutor>::execute"),
    ("  VM execute (legacy)", r"LegacyTransactionExecutor.*::execute\b"),
    ("  mono AptosTransactionExecutor::execute_transaction", r"AptosTransactionExecutor.*execute_transaction"),
    ("  mono session finish/evacuate", r"Session.*::finish|evacuat|FrozenHeap"),
    ("  mono deserialize_into_arena", r"deserialize_into_arena|materialized_value"),
    ("  mono read (ParallelReader)", r"ParallelReader.*read_resource|ParallelReader.*read_group_member"),
    ("  legacy AptosVM::execute_user_transaction", r"AptosVM::execute_user_transaction|execute_single_transaction"),
    ("  mvhashmap fetch_data_and_record_dependency", r"fetch_data_and_record_dependency|fetch_tagged_data_and_record_dependency"),
    ("  mvhashmap read dependency insert", r"RegisteredReadDependencies::insert|register_read|dependencies.*lock"),
    ("  base view get_state_value", r"get_state_value"),
    ("  set_base_value", r"set_base_value|set_raw_base_values"),
    ("  process_resource_output_v2", r"process_resource_output_v2|process_resource_group_output_v2"),
    ("  mvhashmap write_v2", r"::write_v2|VersionedData.*::write"),
    ("  last_input_output.record", r"TxnLastInputOutput.*::record\b"),
    ("  finish_execution", r"SchedulerV2::finish_execution|finish_execution"),
    ("materialize_txn_commit", r"materialize_txn_commit"),
    ("  materialize (mono)", r"TxnOutcome::materialize|outcome.*materialize"),
    ("  record_finalized_output", r"record_finalized_output"),
    ("prepare_and_queue_commit_ready_txn", r"prepare_and_queue_commit_ready_txn"),
    ("next_task", r"SchedulerV2::next_task|::next_task"),
    ("wait_for_dependency", r"wait_for_dependency"),
    ("validate", r"::validate\b|validate_and_commit_delayed_fields"),
    ("finalize_parallel_execution", r"finalize_parallel_execution"),
    ("MVHashMap drop", r"MVHashMap.*drop_in_place|drop_in_place.*MVHashMap|DEFAULT_DROPPER|schedule_drop"),
    ("State::update / layered map", r"State::update|new_layer_impl|SubTreeBuilder|layered_map"),
    ("jemalloc (any frame)", r"_rjem_|je_|rjem_|__rust_alloc|__rust_dealloc"),
    ("siphash (any frame)", r"sip::|SipHasher"),
    ("futex (any frame)", r"futex"),
    ("page fault (any frame)", r"page_fault|handle_mm_fault|clear_page"),
]
COMPILED_INCLUSIVE = [(name, re.compile(pattern)) for name, pattern in INCLUSIVE]


def load(path):
    stacks = []
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt", errors="replace") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            stack, _, count = line.rpartition(" ")
            comm, _, frames = stack.partition(";")
            stacks.append((comm, frames.split(";") if frames else [], int(count)))
    return stacks


def thread_group(comm):
    return re.sub(r"-?\d+$", "", comm)


def categorize(frame):
    for name, pattern in COMPILED:
        if pattern.search(frame):
            return name
    return "other"


def analyze(stacks, worker_prefix):
    by_group = defaultdict(int)
    self_by_cat = defaultdict(int)
    inclusive = defaultdict(int)
    top_self = defaultdict(int)
    worker_samples = 0
    for comm, frames, count in stacks:
        group = thread_group(comm)
        by_group[group] += count
        if not comm.startswith(worker_prefix):
            continue
        worker_samples += count
        leaf = frames[-1] if frames else "[unknown]"
        self_by_cat[categorize(leaf)] += count
        top_self[leaf] += count
        joined = "\n".join(frames)
        for name, pattern in COMPILED_INCLUSIVE:
            if pattern.search(joined):
                inclusive[name] += count
    return {
        "by_group": by_group,
        "self_by_cat": self_by_cat,
        "inclusive": inclusive,
        "top_self": top_self,
        "worker_samples": worker_samples,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--txns", type=float, default=None, help="transactions in the window")
    ap.add_argument("--freq", type=float, default=499.0, help="perf sampling frequency")
    ap.add_argument("--worker-prefix", default="par_exec")
    ap.add_argument("--top", type=int, default=40)
    args = ap.parse_args()

    results = [(path, analyze(load(path), args.worker_prefix)) for path in args.files]
    names = [re.sub(r"\.folded(\.gz)?$", "", p.split("/")[-1]) for p, _ in results]

    def us_per_txn(samples):
        if args.txns is None:
            return None
        return samples * (1e6 / args.freq) / args.txns

    def cell(samples, total):
        share = 100.0 * samples / total if total else 0.0
        us = us_per_txn(samples)
        return f"{share:5.1f}%" if us is None else f"{us:7.2f} ({share:4.1f}%)"

    print("## Samples by thread group (all threads in the execution window)")
    groups = sorted({g for _, r in results for g in r["by_group"]})
    rows = []
    for g in groups:
        rows.append([g] + [cell(r["by_group"].get(g, 0), r["worker_samples"]) for _, r in results])
    print(tabulate(rows, headers=["thread group (us/txn, % of worker)"] + names, tablefmt="github"))
    print()

    print(f"## Worker self time by component ({args.worker_prefix}* threads)")
    cats = [name for name, _ in CATEGORIES] + ["other"]
    rows = []
    for c in cats:
        vals = [r["self_by_cat"].get(c, 0) for _, r in results]
        if sum(vals) == 0:
            continue
        rows.append([c] + [cell(v, r["worker_samples"]) for v, (_, r) in zip(vals, results)])
    rows.append(["total"] + [cell(r["worker_samples"], r["worker_samples"]) for _, r in results])
    print(tabulate(rows, headers=["component"] + names, tablefmt="github"))
    print()

    print("## Worker inclusive time under frames")
    rows = []
    for name, _ in INCLUSIVE:
        vals = [r["inclusive"].get(name, 0) for _, r in results]
        if sum(vals) == 0:
            continue
        rows.append([name] + [cell(v, r["worker_samples"]) for v, (_, r) in zip(vals, results)])
    print(tabulate(rows, headers=["frame"] + names, tablefmt="github"))
    print()

    for (path, r), name in zip(results, names):
        print(f"## Top self symbols: {name}")
        rows = sorted(r["top_self"].items(), key=lambda kv: -kv[1])[: args.top]
        print(
            tabulate(
                [[cell(v, r["worker_samples"]), categorize(k), k[:110]] for k, v in rows],
                headers=["us/txn (share)", "component", "symbol"],
                tablefmt="github",
            )
        )
        print()


if __name__ == "__main__":
    main()
