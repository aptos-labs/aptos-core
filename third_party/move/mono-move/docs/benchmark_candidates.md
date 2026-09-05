# MonoMove benchmark candidates

A menu of benchmark workloads to select from, split into two categories:

1. **Blockchain applications** — modeled on real Move code deployed on Aptos mainnet.
2. **Ports** — established VM benchmarks (JVM lineage, SPEC CPU, EVM) rewritten in Move.

The two categories cover different halves of the VM. Ports exercise the pure
execution engine and nothing else, because they touch no storage, no natives,
and no module boundaries. Blockchain applications are the only way to reach the
storage path, the BCS codec, native calls, generic instantiation at scale, and
linking. Neither category alone is sufficient.

Target is the execution engine, not Block-STM. Contention, abort rates, and
skewed key distributions are deliberately out of scope. Where a workload has a
knob, the knob varies work per transaction, not conflict between transactions.

## What the harness runs today

Eight workloads: `no-op`, `apt-fa-transfer`, `account-generation`,
`modify-global-resource`, `batch100-transfer`, `token-v2-ambassador-mint`,
`liquidity-pool-swap`, `order-book-no-matches1-market`.

Registered but not in the perf list: `liquidity-pool-swap-stable` and nine of
the ten `OrderBook*` variants. Turning these on costs no new Move code and
lights up the 255-iteration Newton loop at `liquidity_pool.move:584`, which the
harness currently never executes.

## VM subsystems

Columns used in the coverage matrix at the end of this document.

| Tag | Subsystem |
| --- | --- |
| Dispatch | Interpreter loop: bytecode fetch, decode, per-instruction overhead |
| Arith | Integer operations, operand width u8–u256, overflow checks, casts |
| Calls | Call frames, argument passing, locals, return, recursion depth |
| Branch | Conditional control flow and its predictability |
| Vector | Bounds checks, element access, in-place mutation, resize |
| Values | Struct-inline vs heap-boxed representation, copy-on-write |
| Generics | Type substitution, instantiation, monomorphization |
| Linking | Module load, cross-module calls, script load and verify |
| Storage | Global resource and table item reads and writes |
| Codec | BCS encode/decode, TypeLayout construction |
| Natives | Transition into Rust for crypto, hashing, randomness |

---

# 1. Blockchain applications

## 1.1 DEX aggregator / router

Reference: `composer_utils` @0x5c5111cf8bde (Panora router family, deployed at
31 addresses), `Panora` @0x1c3206329806 (6 modules, 57 entry fns), `SwapGPT`
@0x509cc56b774b, `OmniSwap` @0x8304621d9c0f, `TappSOR` @0x09280e6cd090.

Corpus path: `mainnet/5/c/5/0x5c5111cf8bde.../composer_utils/`

**What it tests.** This is the strongest single application candidate because
it stresses three subsystems that nothing else reaches together.

`composer_utils` has a router function with **32 type parameters in one
signature**. That is the worst case on chain for type substitution and
instantiation caching. Legacy MoveVM pays substitution cost per call across all
32; MonoMove compiles one specialized body. This is monomorphization stress
from real deployed code rather than a synthetic construction.

The router calls into several distinct DEX protocols, each a separate published
package. So one transaction crosses many module boundaries, exercising module
cache lookup, cross-module call dispatch, and linking. The call chain is the
deepest on chain.

Each leg touches a different pool, so the read set grows with route length, and
routing arithmetic runs between legs. C/IO ratio is 43.1, the highest in the
corpus, though absolute IO is small since it is a routing and math library.

**Knobs.** Hop count (1–5). Number of distinct protocols in the route. Number
of type parameters instantiated.

**Effort.** Medium. The router itself is small; the cost is writing enough
distinct pool backends for the route to be real.

## 1.2 Concentrated liquidity swap (Uniswap V3 shape)

Reference: Hyperion `dex` @0x8b4a2c4bb538 — 32 modules, 135 KB, 132 entry fns.
A full V3 port: `pool_v3`, `tick`, `tick_math`, `tick_bitmap`, `swap_math`,
`liquidity_math`, `bit_math`, `i32`, `i64`, `i128`, `full_math_u128`,
`full_math_u256`.

Corpus path: `mainnet/8/b/4/0x8b4a2c4bb538.../dex/`

**What it tests.** The read set is discovered by computation rather than given
by input. A swap walks the tick bitmap crossing a variable number of
initialized ticks, each a distinct table item read plus a write when crossed,
and one bitmap word read per 256 ticks scanned. The transaction does not know
how much state it needs until it runs. Nothing currently in the harness has
that property.

Alongside that: `get_sqrt_ratio_at_tick` is roughly 20 conditional mul-shifts,
so it is branchy arithmetic; `compute_swap_step` does 512-bit mul-div on
Q64.96, which is cheap in Move since u256 is a VM primitive. Signed integers
(i24 tick, i128 liquidityNet) must be emulated, and that emulation cost is
itself worth measuring.

**Knobs.** Trade size, which sets ticks crossed and therefore read-set width
(roughly 3 to 50 slots). Tick spacing. Position density.

**Effort.** High. 800–1200 lines even after cutting rewards and oracle, and
signed-integer emulation has to come first.

## 1.3 Stableswap Newton iteration

Reference: `Stable` @0xa611a8ba7261 — one module, 27 KB, 79 loops.
`get_D`, `get_D_mem`, `xp_mem`, `amp`, `internal_swap`, amplification ramp.
Protocol identity unattributed.

Corpus path: `mainnet/a/6/1/0xa611a8ba7261.../Stable/sources/stable.move`

**What it tests.** Pure arithmetic throughput with a data-dependent trip count.
`get_D` runs Newton–Raphson on a degree-(n+1) polynomial, typically 5–10
iterations, bounded at 255. The iteration count varies with pool imbalance, so
the loop cannot be unrolled or folded at compile time. Zero storage traffic
during the math.

This is the cleanest compute kernel on mainnet, and unlike the synthetic
kernels it comes with a plausible gas profile and real inputs.

**Knobs.** Pool imbalance, which drives iteration count. Coin count (2 or 3).
Amplification parameter.

**Effort.** Low. 250–350 lines. The existing `liquidity_pool.move:584` `get_y`
is already a bounded Newton loop and can be extended.

## 1.4 CLOB with a hand-rolled AVL tree

Reference: `Econia` @0x040c1a20f392 — 8 modules, 66 KB. `avl_queue` packs an
AVL tree with bit-packed node fields into a `table`; `tablist` is a doubly
linked list over table entries.

Corpus path: `mainnet/0/4/0/0x040c1a20f392.../Econia/`

**What it tests.** This is the closest Move gets to pointer chasing, which is
what makes SPEC's 429.mcf a good benchmark and which Move otherwise cannot
express. Each tree node dereference is a table item read whose key comes from
the previously read node. The reads are sequentially dependent, so they cannot
be batched or overlapped.

Node fields are bit-packed, so every access is shift-and-mask arithmetic on top
of the storage read. Insertion triggers rebalancing, which rewrites several
nodes.

`index_orders_sdk` walks the entire queue in one call, which makes a single
transaction arbitrarily expensive on demand.

**Knobs.** Tree depth (order count). Orders matched per transaction. Insert vs
lookup vs cancel mix.

**Effort.** Medium-high. The AVL implementation is the bulk of it.

## 1.5 Order book on BigOrderedMap

Reference: `aptos-move/framework/aptos-experimental/sources/trading/` (9,128
lines, 21 modules) and `decibel_perp_dex` @0xe6e7f8d3a619. The ladder is
`BigOrderedMap<PriceDescTime, OrderData>`.

Already partly present: `testsuite/benchmark-workloads/packages-experimental/
experimental_usecases/sources/order_book_example.move` (112 lines).

**What it tests.** `BigOrderedMap` is a B-tree spread across storage slots, so
one insert touches several items and a node split rewrites whole nodes. Write
amplification is structural, and read-set width is data-dependent.

This is also the only place the modern feature stack appears together: enums
(decibel declares 207, almost all single-variant `V1 {}` upgrade hedges, so
every field access pays a variant test), closures in the matching callback, and
`aggregator_v2` for open interest.

Caveat worth stating plainly: those features are rare on mainnet. Enums appear
in 0.8% of packages, closures in 0.2%, `big_ordered_map` in 0.4%, and decibel
alone holds roughly half the corpus total of several of them. This benchmark
measures decibel's shape, not mainnet's shape. Build it as a dedicated feature
benchmark, not as the representative application.

**Knobs.** Book depth. Match depth per order. Bulk order width (decibel's
`place_bulk_order` takes four parallel `vector<u64>`).

**Effort.** Low to extend the existing 112-line example; high to model decibel
properly.

## 1.6 Bridge relay — the IO floor

Reference: `layerzero` @0x54ad3d30af77 — 17 modules, C/IO ratio **0.46**, 311
IO sites, 1 loop. Also `endpoint_v2` @0xe60045e20fc2 (0.60).

**What it tests.** Storage path with compute held near zero. This is the
control at the opposite end from snailtracer. Any MonoMove speedup measured
here is attributable to the storage path and the codec, not to the interpreter.

Message delivery reads a nonce, verifies, writes a payload, updates state. Very
little arithmetic between the accesses.

**Knobs.** Payload size. Number of messages per transaction.

**Effort.** Low. The shape is simple; realism is not the point.

## 1.7 Airdrop / batch distribution — write fan-out

Reference: `AniAirdrop` @0xf713bbb607b1 — C/IO **0.05**, 512 IO sites. Also
`Amnis Campaign` @0x70be3af225de (885 IO sites, 228 entry fns, near-zero
compute).

**What it tests.** Write-set width scaling in isolation. One transaction writes
to N recipient accounts with essentially no computation in between.

Directly relevant to MonoMove's write overapproximation: any copy on write
counts as a write, so the write set is an upper bound. This workload makes that
ratio visible and attributable, since there is nothing else going on.

**Knobs.** Recipient count N (1 to 1000). Whether recipients already exist
(create vs update). Payload size per recipient.

**Effort.** Low. Under 100 lines.

## 1.8 Oracle batch update — the native/interpreter split

Reference: `Switchboard` @0x07d7e436f0b2 (42 modules, secp256k1 ×45), `Pyth`
@0xe8fb87c915ba (25 modules).

**What it tests.** The ratio of native call time to interpreter time in a
realistic workload. Verify N signatures through a native, then write N prices.

This matters because MonoMove speeds up the interpreter, not the natives. This
benchmark answers how much of a real transaction MonoMove can actually affect.
Expect a speedup well below the pure-compute kernels, and that is the finding.

**Knobs.** Feed count N. Signature scheme (ed25519 vs secp256k1). Ratio of
verification to state write.

**Effort.** Low-medium.

## 1.9 CDP liquidation sweep — dependent reads

Reference: `ThalaProtocol` @0x6f986d146e4a — 22 modules, C/IO 2.34.
`sorted_vaults` is a Liquity-style sorted linked list; `stability_pool` and
`collateral_auction` sit alongside it.

**What it tests.** Sequentially dependent storage reads. Walking the sorted
vault list to find liquidation candidates means each read's address comes from
the previous read's contents. This is latency-bound rather than
throughput-bound, and it cannot be prefetched.

Complements the AVL queue: same dependency structure, different data layout
(linked list vs tree, resources vs table items).

**Knobs.** List length walked. Number of vaults liquidated per transaction.

**Effort.** Medium.

## 1.10 Lending market — input-dependent read set

Reference: `AavePool` @0x39ddcd9e1a39 (29 modules, 149 view fns) plus
`AaveConfig`, `AaveMath`, `AaveOracle`. Open-source reference:
`github.com/aave/aptos-aave-v3`, Apache-2.0, 49 non-test modules, audited.
Also `MoarMarket` @0xa3afc59243af, `Aries` @0x9770fa9c725c, `joule-core`
@0x2fe576faa841.

**What it tests.** Read-set size that varies per user rather than per
configuration. The health factor loops over a user's collateral bitmask, so
borrow reads N asset configs, N indexes, and N prices where N differs by
account. Liquidation touches two reserves, two users, and two supplies.

The arithmetic is light: a two-slope utilisation curve and a three-term
binomial compounding factor. The interest is in the shape of the state access,
not the math.

Seven entry points with genuinely different cost profiles: `supply`,
`withdraw`, `borrow`, `repay`, `set_use_as_collateral`, `liquidation_call`,
`flash_loan`.

**Knobs.** Collateral assets per user. Reserve count. Entry point mix.

**Effort.** Medium. 400–600 lines. No signed integers needed. Port from the
Apache-2.0 Move reference rather than from Solidity.

## 1.11 ZK verification — the native-dominated control

Reference: `veiled_coin` @0x8767beab4e25, `private_transfers` @0xa242d7b96265
(ristretto255 ×314 each, Bulletproofs range proofs), `plonk-verifier`
@0x1d6b5aec03f4 (crypto_algebra ×172), `groth16_verif` @0x3ea43e8b3d5b,
`Promise-Zk3` @0xf4b48d15f591.

**What it tests.** Almost nothing in the interpreter, and that is the point.
This workload should show a speedup near 1.0x under MonoMove. If it shows more,
something is being measured wrong. If it shows less, native dispatch has
regressed.

Include it as a control, not as a performance target.

**Knobs.** Proof system. Number of proofs verified per transaction.

**Effort.** Low if the framework natives are called directly.

## 1.12 Pathological branch table

Reference: `AmaterasuReveal` — `sources/reveal.move`, 12,925 lines, 91,256-byte
module. One nested if/else rarity table over `randomness::u64_range`,
effectively a hardcoded lookup table compiled into branches.

**What it tests.** Instruction dispatch with no loops, no data locality, and a
large code footprint. It is the worst case for a naive dispatch loop and a
stress test for code size under monomorphization.

Also tests module loading and verification of a very large single function,
which is a path nothing else in the suite touches.

**Knobs.** Table depth. Module size.

**Effort.** Trivial — generate it.

## 1.13 Straight-line unrolled arithmetic

Reference: `MyCoinBagv2` @0xd1b58e44ea11 — 3,622 arithmetic ops, 645 casts,
**zero loops**, 2 modules, 48 KB.

**What it tests.** Per-instruction overhead with no loop caching and no help
from branch prediction. Fully unrolled u256 math. This isolates raw dispatch
cost per bytecode better than any loop-based kernel, because there is no
back-edge to amortize over.

**Knobs.** Instruction count. Operand width. Cast density.

**Effort.** Trivial — generate it.

## 1.14 Arbitrage transaction script

Reference: real MEV traffic. Aptos supports transaction scripts, not just entry
functions.

**What it tests.** The script loading path, which is different from the module
path. A script is compiled into the transaction and loaded per transaction with
no module cache hit, then verified and linked against several published
packages. Nothing in the current suite measures this; the `simple` package
touches scripts only trivially.

A three-hop arbitrage (A→B→C→A across three DEXes) also produces a deep
cross-package call chain, so it overlaps with the aggregator but reaches it
through a different loading path.

**Knobs.** Script size. Number of packages linked against. Hop count.

**Effort.** Low if the pool backends already exist for the aggregator.

## 1.15 NFT mint and marketplace

Reference: `Topaz` @0x2c7bccf7b31b, `souffl3` @0xf6994988bd40,
`WapalLaunchpad` @0x055876a41fba, `rarible-marketplace` @0x465a0051e853.
Marketplace C/IO ratios sit at 0.5–1.2.

**What it tests.** Object creation, address derivation, resource group reads,
and event emission. Partly covered already by `token-v2-ambassador-mint`.

Lower priority: it duplicates existing coverage and the marketplace shape is
close to the bridge relay in profile.

**Knobs.** Collection size. Listings touched per transaction.

**Effort.** Low, but marginal value given existing coverage.

---

# 2. Ports

## 2.1 Are We Fast Yet — the integer-only seven

Source: `github.com/smarr/are-we-fast-yet`, from the DLS'16 paper *Cross-Language
Compiler Benchmarking: Are We Fast Yet?*. Built specifically to test whether an
implementation is highly optimizing, meaning whether it erases abstraction cost.
Restricted to a common language subset so ports stay comparable. Published
per-benchmark dynamic profiles in `docs/metrics.md`.

These seven are integer-only and port without modification. The reason to take
all seven rather than a subset is that they partition the execution engine
almost cleanly — each one dominates a different axis.

| Benchmark | Calls | Max stack | Loop iters | Branch bias | Field reads | Array r / w | Dominates |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Richards | 95.5M | 31 | 1.25M | 0.76 | 217M | 4.25M / 0.94M | calls + fields, least predictable branches |
| Towers | 39.4M | 34 | 601 | 0.93 | 39.2M | 9.8M / 9.8M | pure call-frame cost |
| Sieve | 60K | 26 | 2.01M | 0.98 | 3K | 15M / 48M | array writes, almost no calls |
| Bounce | 17.7M | 30 | 78K | 0.95 | 63.3M | 7.5M / 150K | field reads |
| List | 17.3M | 37 | 4.21M | 0.95 | 57.3M | 0 / 0 | linked structure, zero arrays |
| Permute | 52.0M | 44 | 3.62M | 0.87 | 49.0M | 20.2M / 20.2M | recursion + balanced array r/w |
| Queens | 29.1M | 56 | 1.17M | 0.97 | 34.0M | 26.3M / 8.15M | backtracking, read-heavy arrays |

**Richards.** An OS task scheduler simulation. A kernel passes packets between
device, worker, handler, and idle tasks held in a linked list of task control
blocks, dispatching each polymorphically. Branch bias 0.76 is the lowest in the
suite, so it has the least predictable control flow. Move has no subtyping, so
write it twice — enum match and function values — to price each dispatch
lowering separately.

**Towers.** Towers of Hanoi. 39.4M calls against 601 loop iterations is the
highest call-to-loop ratio in the suite, so this isolates function call
overhead better than anything else available.

**Sieve.** Sieve of Eratosthenes. 48M array writes against 15M reads and only
60K calls. The only write-dominated benchmark in the set, and the exact
complement to Towers.

**Bounce.** Bouncing ball simulation. 63.3M field reads against 17.7M calls is
the highest field-read density. Write-light.

**List.** Linked list manipulation with **zero array operations**. In Move a
linked list must be built from either boxed structs or vector indices, so this
directly probes the struct-inline versus heap-boxed value model. Writing it
both ways makes the delta the cost of boxing.

**Permute.** Recursive permutation generation with array swaps. Read/write
ratio is exactly 1:1, and it is recursive, so it combines call cost with vector
mutation.

**Queens.** N-queens backtracking. 3.2:1 read-to-write on arrays with branch
bias 0.97, so it is read-dominated with predictable branches. Complements
Permute's balance.

**Effort.** Low each, 100–200 lines. They share no infrastructure, so they can
land incrementally.

Excluded from AWFY: Mandelbrot, NBody, and CD are float-heavy. Havlak has a max
stack depth of 1717, which exceeds Move's `CALL_STACK_SIZE_LIMIT = 1024`
(`third_party/move/move-vm/runtime/src/interpreter.rs:1914`), so it would need
restructuring into an explicit worklist. DeltaBlue needs an arena rewrite for
its cyclic object graph. Json is the only megamorphic benchmark in the suite
and is worth adding later for that reason alone.

## 2.2 SPEC CPU

Most of SPEC does not port: no floats, no syscalls, no pointer aliasing, and a
gas bound. These four survive.

**548.exchange2** — recursive sudoku solver, pure integer. The best CPU2017
pick. Deep recursion with backtracking. Same shape as Queens but substantially
larger, so it pushes call depth and working set further.

**456.hmmer, Viterbi core** — already integer log-odds arithmetic. Roughly 41%
loads and 8% branches, making it the load-dominated kernel. It is the direct
complement to Richards: one is call-bound, the other is load-bound. Tests
dynamic-programming table access and index arithmetic hoisting.

**458.sjeng / 531.deepsjeng** — integer alpha-beta chess search. Branchy with
deep recursion and move generation. Needs a fixed-depth subset to fit a gas
budget.

**401.bzip2 / 557.xz, Huffman stage** — integer compression. The Huffman coding
stage alone is the right size to port. Bit manipulation and table-driven
encoding, which nothing else in either category covers.

Explicitly out: 429.mcf's identity is pointer chasing, over 50% of L1 misses in
one loop, and Move cannot express it — candidate 1.4 (Econia's AVL queue over
table items) is the closest available substitute. 470.lbm, 462.libquantum,
464.h264ref, and 403.gcc are float or systems-bound.

**Effort.** Medium each. exchange2 and the Huffman stage are the most
tractable.

## 2.3 EVM

**snailtracer.** A Solidity ray tracer written entirely in fixed-point integer
math. Compute-bound with zero storage access, so it isolates the interpreter
dispatch loop. It became the EVM's standard benchmark because its discriminating
power is documented: revm PR #283 unified instruction function signatures and
moved snailtracer roughly 10% while nothing else moved measurably.

The strategic argument for porting it is that it is the one number other VM
teams already have. It makes MonoMove comparable outside the Aptos ecosystem.

**Fixed-point mandelbrot.** The smaller sibling. Iterate z = z² + c per grid
point until escape, with reals scaled by 2^k so a multiply becomes
`(a * b) >> k`. Sweep u64 / u128 / u256 to price wide-integer multiply, shifts,
and overflow checks against operand width. Zero memory traffic.

**ten-thousand-hashes.** From `evm-bench`. A native hash call in a tight loop.
Serves the same control role as the ZK verifier: it measures the native
transition cost, and MonoMove should barely move it.

**Effort.** Mandelbrot is trivial. snailtracer is medium — the ray tracer math
is substantial but mechanical.

## 2.4 Computer Language Benchmarks Game

**fannkuch-redux.** Generate every permutation of 1..n; for each, repeatedly
read the first element k, reverse the first k elements, and count flips until
the first element is 1. Tight in-place vector reversal on a small array with no
allocation. Highest array-operation density per call of anything in either
category. Tests bounds-check elision, index arithmetic, and loop back-edge
dispatch.

**binary-trees, two representations.** Boxed enum versus index arena. Directly
probes the struct-inline / vector-boxed value model; the delta between the two
is the cost of boxing. Overlaps with AWFY's List, which reaches the same
question through a linked list rather than a tree. Take one or the other unless
both representations matter.

**Effort.** Low.

---

# 3. Coverage matrix

● primary target, ○ secondary.

| Workload | Dispatch | Arith | Calls | Branch | Vector | Values | Generics | Linking | Storage | Codec | Natives |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Blockchain** | | | | | | | | | | | |
| 1.1 Aggregator | | ○ | ● | | | | ● | ● | ○ | | |
| 1.2 CLMM swap | | ● | | ○ | ○ | | | | ● | ○ | |
| 1.3 Stableswap | ● | ● | | ○ | | | | | | | |
| 1.4 AVL queue | | ○ | | ○ | ○ | | | | ● | ○ | |
| 1.5 BigOrderedMap book | | | ○ | ○ | | ○ | | | ● | ● | |
| 1.6 Bridge relay | | | | | | | | | ● | ○ | |
| 1.7 Airdrop fan-out | | | | | | ○ | | | ● | ○ | |
| 1.8 Oracle batch | | | | | | | | | ● | ○ | ● |
| 1.9 CDP sweep | | | | | | | | | ● | | |
| 1.10 Lending | | ○ | | ○ | | | | | ● | ○ | |
| 1.11 ZK verify | | | | | | | | | | | ● |
| 1.12 Branch table | ● | | | ● | | | | ○ | | | ○ |
| 1.13 Unrolled math | ● | ● | | | | | | | | | |
| 1.14 Arb script | | | ○ | | | | ○ | ● | ○ | | |
| 1.15 NFT mint | | | | | | | | | ○ | ○ | ○ |
| **Ports** | | | | | | | | | | | |
| 2.1 Richards | | | ● | ● | | ● | | | | | |
| 2.1 Towers | | | ● | | ○ | | | | | | |
| 2.1 Sieve | ○ | | | | ● | | | | | | |
| 2.1 Bounce | | | ○ | | ○ | ● | | | | | |
| 2.1 List | | | ○ | | | ● | | | | | |
| 2.1 Permute | | | ● | | ● | | | | | | |
| 2.1 Queens | | | ● | ○ | ● | | | | | | |
| 2.2 exchange2 | | | ● | ● | | | | | | | |
| 2.2 hmmer Viterbi | | ○ | | | ● | | | | | | |
| 2.2 sjeng | | | ● | ● | ○ | | | | | | |
| 2.2 bzip2 Huffman | | ● | | ○ | ○ | | | | | | |
| 2.3 snailtracer | ● | ● | ○ | | | | | | | | |
| 2.3 mandelbrot | ● | ● | | | | | | | | | |
| 2.3 ten-thousand-hashes | | | | | | | | | | | ● |
| 2.4 fannkuch-redux | ○ | ○ | | | ● | | | | | | |
| 2.4 binary-trees ×2 | | | ○ | | | ● | | | | | |

Reading the matrix: the ports column block is empty under Generics, Linking,
Storage, Codec, and Natives. Ports cannot reach those paths by construction.
Conversely no blockchain application dominates Calls, Branch, or Values, because
real contracts are shallow and storage-bound. The two categories are
complementary rather than redundant, and a suite drawn from only one leaves
half the engine unmeasured.

Thinnest coverage after selection: Values is reached only by Richards, Bounce,
List, and binary-trees, all ports. If the struct-inline versus heap-boxed split
matters, that is worth a dedicated application-side benchmark as well.

# 4. Suggested selection

Cheapest first, and each step is independently useful.

**Step 0, no new Move code.** Turn on `liquidity-pool-swap-stable` and three or
four unused `OrderBook*` variants.

**Step 1, pure engine.** Mandelbrot with a width sweep, fannkuch-redux, Towers,
Sieve. Four small kernels covering dispatch, arithmetic, calls, and vector
writes. All under 200 lines each.

**Step 2, real applications.** Aggregator (1.1), stableswap (1.3), bridge relay
(1.6), airdrop fan-out (1.7). This spans the compute ceiling to the IO floor
and reaches generics, linking, and the storage path.

**Step 3, breadth.** Richards in both dispatch forms, List, Permute, Queens,
plus Econia's AVL queue (1.4) and the oracle batch (1.8).

**Step 4, the ambitious ones.** CLMM swap (1.2), lending (1.10), snailtracer,
exchange2.

**Controls to include throughout.** ZK verify (1.11) and ten-thousand-hashes
should sit near 1.0x. `no-op` in every sweep, with the delta reported, per
BLOCKBENCH's subtraction convention.

# 5. Methodology notes

Carried over from the survey of existing suites.

- Report compilation, instantiation, and execution separately (Sightglass
  convention). Monomorphization moves cost into compile time, so measuring
  execution alone flatters MonoMove.
- Score code size alongside speed (Embench-IoT convention). It is the honest
  counter-metric for a monomorphizing engine.
- Derive every value from runtime input (CoreMark's rule). Otherwise the
  optimizer folds the benchmark away. This is the flaw that made Dhrystone
  worthless, and it applies with more force to MonoMove than to an interpreter.
- Report per-workload results, not an aggregate. Which entry point regressed
  matters more than the geomean.

# 6. Caveats

- Corpus counts other than view and attribute counts are regex counts over
  decompiler output. Ratios between packages are meaningful; absolute values
  are not.
- `Stable` @0xa611a8ba7261 is unattributed. The Tapp-hook reading is a guess.
- SPEC and AWFY characterizations come from published sources, not from runs in
  this repo. Verify before citing externally.
- Recursion frequency across the corpus was never measured; it needs a
  call-graph pass. `scripts/decompile_modules/src/bin/init_module_callgraph.rs`
  in the move-modules repo is the starting point.
- Corpus snapshot is at mainnet block height 1,019,832,938.
