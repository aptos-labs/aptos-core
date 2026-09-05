# bench_clob

A central limit order book whose price levels live in a bit-packed AVL queue,
modeled on the one in [Econia](https://github.com/econia-labs/econia).

## Why this workload

This is the closest Move gets to pointer chasing. A tree node is a single
`u128` in a `Table<u64, u128>`, and the id of the next node to visit is a
14-bit field inside the `u128` that was just read. So a descent is a chain of
table reads where the address of read `k + 1` is not known until read `k` has
returned and been shifted and masked. Nothing can be reordered or prefetched.
No other benchmark in the suite has that dependency structure.

The shift-and-mask decoding is not incidental. It is layered on top of every
table read on purpose, so the measurement covers both the storage read and the
integer work needed to interpret it. The packing is never flattened into
separate fields and node reads are never cached in a local map.

Collateral is real fungible assets, not an internal balance ledger. Market
accounts hold `Object<FungibleStore>` at named object addresses and funding
goes through `primary_fungible_store`, which puts derived-object address
computation and resource-group reads on the deposit path.

## Modules

| Module | Contents |
| --- | --- |
| `bench::clob_avl_queue` | The AVL queue: bit-packed tree and list nodes, free-list stacks, rotations, in-order walk. |
| `bench::clob_market` | Markets, market accounts, matching, and the order entry points. |
| `bench::clob_mock_fa` | Base and quote as vanilla fungible assets. |

### Layout

Widths and caps match the reference. The field order within each word is this
package's own and is documented in the `clob_avl_queue` module header, which
carries the full tables. Summarised:

| Word | Fields |
| --- | --- |
| Queue header (`u128`) | Head key 32, head list node 14, tail key 32, tail list node 14, tree stack top 14, list stack top 14, sort order 1 |
| Tree node (`u128`) | Key 32, parent 14, left 14, right 14, list head 14, list tail 14, next inactive 14, left height 5, right height 5 |
| List node (`u128`) | Prev 14, prev-is-tree flag 1, next 14, next-is-tree flag 1 |
| Access key (`u64`) | Key 32, list node 14, tree node 14, sort order 1 |

Node ids are 14 bits, so at most 16383 tree nodes and 16383 list nodes, with 0
reserved for null. Insertion keys are 32 bits. A node stores each child's
subtree height plus one, 0 meaning the child is absent, which fits in 5 bits
because an AVL tree of 16383 nodes is at most 18 tall.

The tree root does not fit in the remaining 7 bits of the header, so it is a
separate `u64` field on the struct.

A market order id is a `u128`: the high 64 bits are a per-market counter, the
low 64 bits are the AVL access key. Node ids are recycled, so an access key
alone is not a stable identity; the counter makes the id unique over time and
`cancel_order` rejects a stale one with `E_STALE_ORDER_ID` (10). The sort-order
bit inside the access key is what tells `cancel_order` which side to look on,
so the caller does not pass the side.

## Initialization recipe

Named address `bench` is `0xB0`. `admin` must be that address.

1. `clob_mock_fa::create_asset_entry(admin, symbol, decimals)` twice, once for
   base and once for quote. Each metadata object is a named object seeded by
   `symbol`; recover it later with `clob_mock_fa::asset(admin_addr, symbol)`.
2. `clob_market::register_market(admin, base, quote, lot_size, tick_size)`.
   Market ids count from one. The first call also publishes the registry.
3. Per user: `clob_mock_fa::mint_entry(admin, asset, user, amount)` for both
   assets, then `clob_market::register_market_account(user, market_id)`, then
   `clob_market::deposit(user, market_id, asset, amount)` for both assets.
   Registration creates the two collateral stores, so it must precede the
   deposits.
4. `clob_market::seed_book(admin, market_id, n_bids, n_asks, base_price,`
   `spread, seed)` to fill the book. `admin` needs a market account and enough
   collateral for every seeded order, so run step 3 for `admin` first.

`seed_book` posts directly without matching, so bids and asks never cross and
all `n_bids + n_asks` orders rest.

## Entry points

| Entry point | Signature |
| --- | --- |
| `register_market` | `(admin, base, quote, lot_size, tick_size)` |
| `register_market_account` | `(user, market_id)` |
| `deposit` | `(user, market_id, asset, amount)` |
| `place_limit_order` | `(user, market_id, side, price, size)` |
| `place_market_order` | `(user, market_id, side, size)` |
| `cancel_order` | `(user, market_id, market_order_id)` |
| `change_order_size` | `(user, market_id, market_order_id, new_size)` |
| `seed_book` | `(admin, market_id, n_bids, n_asks, base_price, spread, seed)` |
| `run` | `(_s, market_id, side, limit, expected)` |

`side` is `true` for bids and `false` for asks; `clob_market::bid_side()` and
`ask_side()` return them. `price` is in ticks and `size` in lots, so the base
amount is `size * lot_size` and the quote amount `size * price * tick_size`.

Entry functions cannot return values, so three have `_id`-suffixed public
non-entry twins that do: `register_market_id` returns the market id, and
`place_limit_order_id` and `change_order_size_id` return the market order id
(zero from `place_limit_order_id` when the order filled completely).

The kernel and its transaction wrapper follow the suite convention:

```move
public fun index_orders(market_id: u64, side: bool, limit: u64): u64
public entry fun run(_s: &signer, market_id: u64, side: bool, limit: u64, expected: u64)
```

`index_orders` walks one side in price order via `head_access_key` and
`next_access_key`, folding `acc = (acc * 1000003 + price + size) % 1000000007`
over at most `limit` orders. That walk is the read-only form of the dependent
load chain: each step decodes the current list node to find the next one, and
crossing a price level goes back up through the tree. `run` aborts with
`E_BAD_RESULT` (13) if the checksum does not match.

Six `#[view]` functions read state without changing it: `best_price`,
`book_height`, `n_orders`, `resting_size`, `account_state`, `store_balances`.

## Knobs

Every knob is a runtime argument, `expected` included. No compile-time constant
gates work that the optimizer could fold away.

| Knob | Where | Effect |
| --- | --- | --- |
| `n_bids` / `n_asks` | `seed_book` | Orders per side, so tree size and therefore descent depth |
| `base_price`, `spread` | `seed_book` | Where the book sits and how wide the touch is |
| `seed` | `seed_book` | Which prices the LCG picks, so the tree shape |
| `limit` | `index_orders`, `run` | Orders walked, which is what makes one transaction arbitrarily expensive |
| `price` | `place_limit_order` | Whether the order crosses, and how deep it descends to rest |
| `size` | `place_market_order` | How many price levels a taker eats before it stops |
| `lot_size`, `tick_size` | `register_market` | Collateral per lot; changes the arithmetic, not the traversal |

Prices are jittered rather than evenly spaced. Evenly spaced insertions would
build an unnaturally regular tree; the LCG spread over
`n * 4 + 1` ticks per side gives a mix of fresh price levels and duplicates
landing on an existing level's list.

### Measured book depth

With `lot_size = 100`, `tick_size = 10`, `base_price = 100000`, `spread = 100`,
`seed = 42`:

| N per side | Bid tree height | Ask tree height | `index_orders(ASK, N)` | `index_orders(BID, N)` |
| --- | --- | --- | --- | --- |
| 32 | 5 | 4 | 267044062 | 798828790 |
| 256 | 8 | 8 | 621841564 | 783254965 |
| 2048 | 12 | 12 | 715030210 | 560303969 |

Height is the number of dependent table reads a lookup at the deepest price
level costs. Read those checksums as expected values for `run` at those exact
knob settings; any change to `base_price`, `spread`, `seed`, or N changes them.

## Suggested mix

- Non-crossing `place_limit_order` at N = 32 / 256 / 2048 per side. One insert,
  one descent, one retrace.
- `place_market_order` sized to sweep 1, 4, and 16 price levels. Each level
  costs a head lookup, a fill, and a removal that may rotate.
- `cancel_order` on a resting order. A removal from the middle of a level is
  list surgery only; a removal that empties a level goes through the BST delete
  and the retrace.
- `index_orders` with `limit` at N, N/4, and 1, which isolates the walk from the
  mutation.

## Tests

80 tests, 53 in `clob_avl_queue` and 27 in `tests/clob_market_tests.move`.

```bash
./target/debug/aptos move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_clob \
  --skip-fetch-latest-git-deps
```

Coverage:

- LCG fuzz, four variants (ascending, descending, duplicate-heavy, wide keys),
  a few hundred operations each, re-asserting every AVL invariant after each
  step: BST order, the balance factor at every node, stored heights against
  recomputed ones, parent back pointers, list head and tail flags, the cached
  header head and tail, node counts, and height against the 18 cap.
- Round-trips at both boundaries of all 21 bit fields, one test per field.
- All four rotations on insert and on remove: LL, RR, LR, RL.
- `pop_head` sort order in both directions, plus the walk in both directions.
- Matching: partial fill, crossing order resting its remainder, fully filled
  order resting nothing, a limit order stopping at its own price, and a market
  order stopping when the book empties.
- `cancel_order` removing exactly one order and leaving its level intact,
  wrong-owner rejection, and stale-id rejection.
- End to end: two assets, a market, two funded users, seeding, matching,
  cancelling, resizing, and asset conservation across the whole run.

## Provenance

Modeled on the AVL queue in [Econia](https://github.com/econia-labs/econia).
Econia is licensed under the Business Source License 1.1, which is incompatible
with this repository's Apache-2.0 license. No code was copied. This package was
written from the data-structure description: a BST on a 32-bit insertion key
where each node owns a doubly linked list, both node kinds bit-packed into one
`u128` in a table, node ids 14 bits, free-list stacks for reuse.

The reference was read to understand the structure and to check behavior after
the fact, never to transcribe. Numeric facts are facts and match: the node-id
width, the node count cap, the insertion key width, the height cap and its
encoding. Field order within each word, the header layout, the access key
layout, the retrace loop, and the entire market layer are this package's own.

## Parity

Parity is asserted by test, not by inspection. Ten `test_parity_*` tests in
`clob_avl_queue` each encode a fact taken from the reference's own
documentation, mostly its worked ASCII diagrams, and check this implementation
against it. What each one checks:

| Test | Behavior checked | How |
| --- | --- | --- |
| `test_parity_caps` | Numeric constants | Asserts node cap 16383 equals the 14-bit mask `0x3fff`, insertion key mask `0xffffffff`, height mask `0x1f`, height cap 18, `NIL = 0`, and the `ASCENDING`/`DESCENDING` and `LEFT`/`RIGHT` polarities |
| `test_parity_height_progression` | Height convention | Replays the reference's height diagram, inserting keys 4, 5, 3, 1 and asserting tree height 0, 1, 1, 2 after each |
| `test_parity_height_table` | Per-node height encoding | Builds the reference's tree rooted at 2 with children 1 and 3 and right grandchild 4, then asserts each node's `(left, right)` stored heights are `(0,0)`, `(1,2)`, `(0,1)`, `(0,0)` and the root's height is 2 |
| `test_parity_insert_sequence` | Insert, rebalance, duplicate handling | Replays the reference's insertion diagram `(3,9), (4,8), (5,7), (3,6), (5,5)`: the third insert must rotate 4 to the root with children 3 and 5, duplicates must append at their level's tail giving key 3 the list `[9, 6]` and key 5 the list `[7, 5]`, and a full drain must yield 9, 6, 8, 7, 5 |
| `test_parity_remove_ascending_head` | Removal case: head | On the reference's removal tree, key 2 holding `[3, 4]` and key 1 holding `[5, 6]`, drains from the head and asserts 5, 6, 3 |
| `test_parity_remove_ascending_tail` | Removal case: tail | Same tree, drains from the tail and asserts 4, 3, 6 |
| `test_parity_remove_descending_head` | Removal case: head, reversed | Same tree as a descending queue, drains from the head and asserts 3, 4, 5 |
| `test_parity_remove_descending_tail` | Removal case: tail, reversed | Same tree as a descending queue, drains from the tail and asserts 6, 5, 4 |
| `test_parity_remove_mid_list` | Removal case: neither head nor tail | Removes the middle element of a three-element level and asserts the header's cached head and tail and the tree node itself are untouched |
| `test_parity_access_key_reuse` | Node id recycling | Removes an element, inserts another, and asserts the new element gets the freed node id, so the same access key names a different element than before |

The four removal tests correspond to the four cases the reference enumerates:
the removed element is the queue head, the queue tail, both, or neither.

Beyond those ten, the invariant checker used by the fuzz tests asserts the AVL
properties the reference relies on but does not diagram: the balance factor
stays in `[-1, 1]` at every node, stored heights equal recomputed subtree
heights, and the tree height never exceeds 18 at the node cap.

Two behaviors deliberately do not match, both documented above and in the
module header. The access key packs its fields in a different order than the
reference does, at the same widths. The tree root is a separate struct field
rather than a header bit field, because this header layout does not leave room
for it.
