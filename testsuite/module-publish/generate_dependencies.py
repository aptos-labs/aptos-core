#!/usr/bin/env python3

import argparse, random, textwrap, pathlib
import networkx as nx
import matplotlib.pyplot as plt

"""
Generate synthetic Move module graphs for module loading benchmarking.
================================================================

This utility writes one `.move` source file per graph shape, placing all modules (M0 ... M<n‑1>)
in that file under the address you choose. Each root module `M0` contains an entry function:
```
public entry fun call(&signer, ...) { ...} 
```
so the graph can be exercised directly from a transaction.


## Supported shapes
  - chain          Linear M0 → M1 → …
  - star           One root, many leaves
  - tree           k‑ary tree (   --fanout   )
  - dag            Random directed acyclic graph. User can specify the probability of edges [0-1].

For DAGs, it is possible to specify a random acyclic path to execute. This way both loader and
the VM can be benchmarked.


## Visualisation

Add `--viz` to also produce `.png` in the output folder.


## Example usage

```
>>> python3 generate_dependencies.py chain --n 64 --addr 0xABCD --out graphs
>>> python3 generate_dependencies.py dag --n 256 --prob 0.08 --path-len 20 --viz
```
"""


################################################################################
# Graph construction
################################################################################

# Generates a chain graph: a linear sequence of nodes.
def gen_chain(num_nodes):
    return {i: [i + 1] if i < num_nodes - 1 else [] for i in range(num_nodes)}


# Generates a star graph: one node pointing to many others.
def gen_star(num_leaves):
    assert num_leaves >= 2, "Star graph requires at least 2 nodes"
    return {0: list(range(1, num_leaves)), **{i: [] for i in range(1, num_leaves)}}


# Generates a tree with specified number of nodes. Fanout controls tree's breadth.
def gen_tree(num_nodes, fanout):
    return {i: [i * fanout + k + 1 for k in range(fanout)
                if i * fanout + k + 1 < num_nodes] for i in range(num_nodes)}


# Generates a random DAG with edges added at specified probability.
def gen_random_dag(num_nodes, p_edge, seed):
    random.seed(seed)
    adj = {i: [] for i in range(num_nodes)}
    for i in range(num_nodes):
        for j in range(i + 1, num_nodes):
            if random.random() < p_edge:
                adj[i].append(j)
    return adj


################################################################################
# Runtime path selection
################################################################################

# Returns a random acyclic path in the graph of at most the desired length (it is possible that the
# path may reach a node with no outgoing edges). The path always starts at 0.
def random_path(adj, path_len):
    path = [0]
    while len(path) < path_len and adj[path[-1]]:
        path.append(random.choice(adj[path[-1]]))
    return path


################################################################################
# Move module rendering
################################################################################

NO_OP = "    public fun op(): u64 { 1 }"


def as_entry_function(body: str, entry_args: str) -> str:
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


# Emits .move file containing modules.
def render_module(addr: str, idx: int, deps, shape: str, path_next, is_tail: bool) -> str:
    uses = "\n".join([f"    use {addr}::M{d};" for d in deps])
    if deps:
        uses = uses + "\n\n"
    body = NO_OP

    if shape == "chain":
        if deps:
            body = f"""\
    public fun op(depth: u64): u64 {{
        if (depth == 0) {{ 0 }} else {{
            M{deps[0]}::op(depth - 1) + 1
        }}
    }}"""
        if idx == 0:
            body = as_entry_function(body, "depth: u64")

    elif shape == "star":
        if idx == 0:
            # Enough for allowed-size graphs:
            branches = []
            calls = []
            for d in deps:
                calls.append(f"            let _ = M{d}::op();")
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
            body = as_entry_function(body, "b: u64")

    elif shape == "tree":
        if deps:
            calls = "\n".join([f"        sum = sum + M{d}::op();" for d in deps])
            body = f"""\
    public fun op(): u64 {{
        let sum = 1u64;
{calls}
        sum
    }}"""
        if idx == 0:
            body = as_entry_function(body, "")

    else:
        if idx in path_next:
            nxt = path_next[idx]
            body = f"""\
    public fun op(): u64 {{
        M{nxt}::op() + 1
    }}"""

        elif is_tail:
            body = NO_OP

        if deps:
            calls = "\n".join([f"        sum = sum + M{d}::op();" for d in deps])
            body = body + "\n\n" + f"""\
    public fun dummy(): u64 {{
        let sum = 1u64;
{calls}
        sum
    }}"""

        if idx == 0:
            body = as_entry_function(body, "")

    # -------------------------------------------------------------------------
    return f"""module {addr}::M{idx} {{\n{uses}{body}\n}}"""


################################################################################
# Visualisation helper
################################################################################

def visualise(adj, path_edges, out_file: pathlib.Path):
    g = nx.DiGraph()
    for u, vs in adj.items():
        for v in vs:
            g.add_edge(u, v)

    pos = nx.spring_layout(g, seed=50)
    plt.figure(figsize=(10, 10))

    nx.draw_networkx_edges(g, pos, edge_color="#BBBBBB", arrowsize=8, width=1)
    if path_edges:
        nx.draw_networkx_edges(g, pos, edgelist=path_edges, edge_color="tab:orange", arrowsize=12, width=2.5)

    path_nodes = {u for u, _ in path_edges} | {v for _, v in path_edges}
    node_colors = ["tab:orange" if n in path_nodes else "tab:blue" for n in g.nodes]
    nx.draw_networkx_nodes(g, pos, node_color=node_colors, node_size=300)
    nx.draw_networkx_labels(g, pos, font_size=8)

    plt.axis("off")
    plt.tight_layout()
    plt.savefig(out_file)
    plt.close()


################################################################################
# File writer
################################################################################

def write_graph(adj, addr, filename, shape, path_next, path_tail, out_dir: pathlib.Path, do_viz):
    out_dir.mkdir(parents=True, exist_ok=True)

    modules = []
    for idx, deps in adj.items():
        mod = render_module(addr, idx, deps, shape, path_next, is_tail=(idx == path_tail))
        modules.append(mod)

    outs = [str(v) for v in path_next.values()]
    if outs:
        path_str = f"Path of size {len(outs)}: 0 --> " + " --> ".join(outs)
    else:
        path_str = "Path not selected"

    header = textwrap.dedent(f"""
        //////////////////////////////////////////////////////////////////////
        // Auto‑generated {shape} graph  –  {len(adj)} modules.
        // {path_str}
        //////////////////////////////////////////////////////////////////////
        """).rstrip()

    src_path = out_dir / f"{filename}.move"
    src_path.write_text(header + "\n\n" + "\n\n".join(modules) + "\n")

    if do_viz:
        path_edges = [(u, v) for u, v in path_next.items()]
        if path_tail != -1:
            path_edges.append((list(path_next.keys())[-1], path_tail))
        visualise(adj, path_edges, out_dir / f"{filename}.png")


################################################################################
# CLI entry point
################################################################################

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        description="""Generate Move module dependency graphs for benchmarking.""")

    p.add_argument("shape", choices=["chain", "star", "tree", "dag"],
                   help="Graph topology to emit.")
    p.add_argument("--n", type=int, default=8,
                   help="Total number of modules (nodes) in the graph.")
    p.add_argument("--addr", default="0xABCD",
                   help="Hex account address to assign to every module.")
    p.add_argument("--out", default="src/packages/dependencies", metavar="DIR",
                   help="Output folder for .move (and optional .png) files.")

    p.add_argument("--fanout", type=int, default=2,
                   help="k‑ary fan‑out for *tree* shape.")
    p.add_argument("--prob", type=float, default=0.15,
                   help="Edge probability for random *dag* shape (0–1).")
    p.add_argument("--seed", type=int, default=42,
                   help="RNG seed – set for reproducible DAG edges & paths.")
    p.add_argument("--path-len", type=int, default=16,
                   help="Length of runtime call path for dage. If 1 - no path.")

    p.add_argument("--viz", action="store_true",
                   help="Also emit <shape>.png visualising the graph & call path.")

    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()
    random.seed(args.seed)

    if args.shape == "chain":
        adj = gen_chain(args.n)
        filename = f"chain-{args.n}"
    elif args.shape == "star":
        adj = gen_star(args.n + 1)
        filename = f"star-{args.n}"
    elif args.shape == "tree":
        adj = gen_tree(args.n, args.fanout)
        filename = f"{args.fanout}-tree-{args.n}"
    elif args.shape == "dag":
        adj = gen_random_dag(args.n, args.prob, args.seed)
        filename = f"dag-{args.n}-p{args.prob}"
    else:
        raise ValueError("Unknown shape")

    path_next = {}
    tail = -1
    if args.shape in {"dag"} and args.path_len > 1:
        pth = random_path(adj, min(args.path_len, args.n))
        path_next = {pth[i]: pth[i + 1] for i in range(len(pth) - 1)}
        tail = pth[-1]
        filename = filename + f"-path{len(pth)}"

    write_graph(adj, args.addr, filename, args.shape, path_next, tail, pathlib.Path(args.out), args.viz)

    print(f"Generated '{args.out}/{filename}.move'")
    if args.viz:
        print(f"Added dependency graph visualization, see {filename}.png")
