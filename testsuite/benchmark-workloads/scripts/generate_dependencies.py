#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import argparse
import pathlib
import random


################################################################################
# Graph construction
################################################################################

# Generates a chain graph: a linear sequence of nodes.
def generate_chain(num_nodes):
    return {i: [i + 1] if i < num_nodes - 1 else [] for i in range(num_nodes)}


# Generates a star graph: one node pointing to many others.
def generate_star(num_leaves):
    assert num_leaves >= 2, "Star graph requires at least 2 nodes"
    return {0: list(range(1, num_leaves)), **{i: [] for i in range(1, num_leaves)}}


# Generates a tree with specified number of nodes. Fanout controls tree's breadth.
def generate_tree(num_nodes, fanout):
    return {i: [i * fanout + k + 1 for k in range(fanout)
                if i * fanout + k + 1 < num_nodes] for i in range(num_nodes)}


# Generates a random DAG with edges added at specified probability.
def generate_random_dag(num_nodes, p_edge, seed):
    random.seed(seed)
    adj = {i: [] for i in range(num_nodes)}
    for i in range(num_nodes):
        for j in range(i + 1, num_nodes):
            if random.random() < p_edge:
                adj[i].append(j)
    return adj


################################################################################
# Move package generation
################################################################################

ADDRESS = "0xABCD"
NO_OP = "    public fun op(): u64 { 1 }"


def emit_move_toml(package_name):
    return f"""\
[package]
name = "{package_name}"
version = "0.0.0"

[dependencies]
    """


def emit_entry_function(body, entry_args):
    if entry_args:
        entry = f"""\
    public entry fun call(_account: &signer, {entry_args}) {{
        let _ = op({entry_args.split(':')[0].strip()});
    }}"""
    else:
        entry = f"""\
    public entry fun call(_account: &signer) {{
        let _ = op();
    }}"""
    return entry + "\n\n" + body


def emit_chain(name, idx, deps):
    if deps:
        assert len(deps) == 1
        body = f"""\
    public fun op(depth: u64): u64 {{
        if (depth == 0) {{ 
            0
        }} else {{
            {name}_{deps[0]}::op(depth - 1) + 1
        }}
    }}"""
    else:
        body = "    public fun op(_depth: u64): u64 { 1 }"

    if idx == 0:
        body = emit_entry_function(body, "depth: u64")

    return body


def emit_star(name, idx, deps):
    if idx == 0:
        # Enough for allowed-size graphs:
        branches = []
        calls = []
        for d in deps:
            calls.append(f"            let _ = {name}_{d}::op();")
            if d == 64 or d == 128 or d == 256 or d == 512 or d == len(deps):
                calls = "\n".join(calls)
                branches.append(f"        if (b >= {d}) {{\n{calls}\n        }};")
                calls = []
        branches = "\n".join(branches)
        body = f"""\
    public fun op(b: u64): u64 {{
{branches}
        0
    }}"""
        body = emit_entry_function(body, "b: u64")
    else:
        body = NO_OP
    return body


def emit_tree(name, idx, deps):
    if deps:
        calls = "\n".join([f"        sum = sum + {name}_{d}::op();" for d in deps])
        body = f"""\
    public fun op(): u64 {{
        let sum = 1u64;
{calls}
        sum
    }}"""
    else:
        body = NO_OP

    if idx == 0:
        body = emit_entry_function(body, "")
    return body


def emit_dag(name, idx, deps):
    body = NO_OP
    if deps:
        calls = "\n".join([f"        sum = sum + {name}_{d}::op();" for d in deps])
        body = body + "\n\n" + f"""\
    public fun dummy(): u64 {{
        let sum = 1u64;
{calls}
        sum
    }}"""

    if idx == 0:
        body = emit_entry_function(body, "")
    return body


def emit_module(shape, name, idx, deps):
    uses = "\n".join([f"    use {ADDRESS}::{name}_{d};" for d in deps])
    if deps:
        uses = uses + "\n\n"

    if shape == "chain":
        body = emit_chain(name, idx, deps)
    elif shape == "star":
        body = emit_star(name, idx, deps)
    elif shape == "tree":
        body = emit_tree(name, idx, deps)
    else:
        assert shape == "dag"
        body = emit_dag(name, idx, deps)

    return f"""module {ADDRESS}::{name}_{idx} {{\n{uses}{body}\n}}"""


################################################################################
# CLI
################################################################################

def parse_args():
    p = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        description="""Generate Move package with modules forming different dependency graphs.""")

    p.add_argument("shape",
                   choices=["chain", "star", "tree", "dag"],
                   help="Graph topology to emit.")
    p.add_argument("--num-nodes",
                   type=int,
                   default=8,
                   help="Total number of modules in the package (nodes in the graph).")

    p.add_argument("--package-name",
                   help="Name of the package. All modules in the package will be called with the following format: "
                        "{package_name}_{idx}. Here, indices used to uniquely identify modules based on index in the "
                        "generated graph. Entrypoint module always has index 0.")
    p.add_argument("--out-dir",
                   metavar="DIR",
                   required=True,
                   help="Output folder for generated Move packages.")

    # Tree only.
    p.add_argument("--tree-fanout",
                   type=int,
                   default=2,
                   help="k‑ary fan‑out for trees.")

    # DAG only.
    p.add_argument("--dag-prob-edge",
                   type=float,
                   default=0.15,
                   help="Probability for DAG edges. Should be between 0 and 1.")
    p.add_argument("--dag-seed",
                   type=int,
                   default=42,
                   help="Seed for reproducible DAG edges.")

    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()
    random.seed(args.dag_seed)

    if args.shape == "chain":
        adj = generate_chain(args.num_nodes)
        filename = f"chain-{args.num_nodes}"
    elif args.shape == "star":
        adj = generate_star(args.num_nodes + 1)
        filename = f"star-{args.num_nodes}"
    elif args.shape == "tree":
        adj = generate_tree(args.num_nodes, args.tree_fanout)
        filename = f"{args.tree_fanout}-tree-{args.num_nodes}"
    elif args.shape == "dag":
        adj = generate_random_dag(args.num_nodes, args.dag_prob_edge, args.dag_seed)
        filename = f"dag-{args.num_nodes}-p{args.dag_prob_edge}-s{args.dag_seed}"
    else:
        raise ValueError(f"Unknown shape: {args.shape}")

    package_dir = pathlib.Path(args.out_dir)
    package_dir.mkdir(parents=True, exist_ok=True)
    move_toml_path = package_dir / "Move.toml"

    move_toml = emit_move_toml(args.package_name)
    move_toml_path.write_text(move_toml + "\n")

    modules = []
    for idx, deps in adj.items():
        mod = emit_module(args.shape, args.package_name, idx, deps)
        modules.append(mod)

    sources_dir = package_dir / "sources"
    sources_dir.mkdir(parents=True, exist_ok=True)

    move_file_path = sources_dir / f"{filename}.move"
    move_file_path.write_text("\n\n".join(modules) + "\n")
