# bench_clmm

A concentrated-liquidity AMM: ticks, a tick bitmap, positions, fee growth
accumulators and real fungible-asset vaults.

## What it stresses

- **Wide `u256` arithmetic.** Every price step runs `mul_div` over 256-bit
  products, and the tick/price conversion does 19 conditional multiply-shifts
  plus a 32-iteration squaring loop.
- **Native signed integers.** Ticks are `i32`, per-tick liquidity deltas are
  `i128`, and the log-to-tick conversion goes through `i256`. Move has no
  signed shifts, so the fixed-point code has to route bit work through
  unsigned types and cast at the boundary.
- **Sparse table lookups keyed by signed and struct keys.** `Table<i16, u256>`
  for the bitmap, `Table<i32, TickInfo>` for ticks, and
  `Table<PositionKey, PositionInfo>` for positions, where `PositionKey` is a
  three-field struct.
- **A read set that grows with price movement, not with pool size.** A swap
  visits one tick per crossing. Swapping across 3, 10 or 50 ticks changes the
  number of table reads without changing anything else.
- **Resource groups and derived object addresses.** Tokens are real fungible
  assets. Every transfer reads an object's resource group and computes a
  derived primary-store address.

## Initialization

1. `clmm_mock_fa::create_asset(admin, b"TKA", 8)` and again for `b"TKB"`.
   Assets are named objects seeded by their symbol, so
   `clmm_mock_fa::metadata(admin, b"TKA")` recovers the handle later.
2. `clmm_pool::create_pool(admin, token_a, token_b, fee_rate, tick_spacing,
   initial_sqrt_price)`. Pass `clmm_tick_math::q96()` to start at tick 0. The
   pool is a named object; `clmm_pool::pool_address(admin, token_a, token_b,
   fee_rate, tick_spacing)` gives its address.
3. `clmm_mock_fa::mint(admin, token, to, amount)` to fund LPs and traders.
   Minting goes into the primary fungible store.
4. `clmm_pool::seed_positions(admin, pool_id, n_positions, width_ticks, seed)`
   opens `n_positions` ranges of `width_ticks`, jittered around the current
   tick by the house generator so the bitmap ends up sparse rather than
   contiguous.

## Entry points

| Function | Effect |
| --- | --- |
| `clmm_mock_fa::create_asset_entry(admin, symbol, decimals)` | Create a test token. |
| `clmm_mock_fa::mint(admin, asset, to, amount)` | Fund an address. |
| `clmm_mock_fa::faucet(asset, to, amount)` | Same, without the owner check. |
| `clmm_pool::create_pool(admin, token_a, token_b, fee_rate, tick_spacing, initial_sqrt_price)` | Open a pool. |
| `clmm_pool::mint(owner, pool_id, tick_lower, tick_upper, liquidity)` | Add liquidity, pulling both tokens as the price dictates. |
| `clmm_pool::burn(owner, pool_id, tick_lower, tick_upper, liquidity)` | Remove liquidity; the principal is credited to the position. |
| `clmm_pool::poke(owner, pool_id, tick_lower, tick_upper)` | Settle fees without changing liquidity. |
| `clmm_pool::collect(owner, pool_id, tick_lower, tick_upper, requested_a, requested_b)` | Withdraw what the position is owed. |
| `clmm_pool::swap_exact_in(trader, pool_id, a_to_b, amount_in, sqrt_price_limit)` | Sell a fixed amount. |
| `clmm_pool::swap_exact_out(trader, pool_id, a_to_b, amount_out, sqrt_price_limit)` | Buy a fixed amount. |
| `clmm_pool::seed_positions(admin, pool_id, n_positions, width_ticks, seed)` | Open a batch of jittered positions. |
| `clmm_pool::bench_onboard(owner, pool_id, tick_lower, tick_upper, liquidity, fund_amount)` | Faucet both tokens and open one position. |
| `clmm_pool::bench_swap_in(trader, pool_id, a_to_b, amount_in)` | Faucet the input side, then swap to the price bound. |
| `clmm_pool::bench_rebalance(owner, pool_id, tick_lower, tick_upper, liquidity, fund_amount)` | Mint, burn the same liquidity, and collect. |

Views: `state`, `liquidity`, `current_tick`, `sqrt_price`, `vault_balances`,
`position_liquidity`, `position_tokens_owed`, `tick_is_initialized`,
`tick_liquidity_net`, `tick_liquidity_gross`.

## Transaction harness

The `bench_*` entry points exist so a harness driving thousands of independent
senders can reach every code path without observing chain state:

1. Publisher, once: create both tokens, mint itself a large balance,
   `create_pool`, `mint` a wide backstop position, then `seed_positions`.
2. Each account, once: `bench_onboard(pool_id, lower, upper, liquidity, fund)`.
   Deriving the range from the account address lets the mix recompute it later
   without tracking anything.
3. Steady state: `bench_swap_in`, `bench_rebalance`, `poke`, `collect`.

Two things keep every transaction succeeding. A swap aborts with `EEMPTY_SWAP`
when it cannot move at all, so the publisher's backstop position has to span
much wider than the mix can push the price. And every transaction that pays
into the pool faucets first, so a long run of swaps in one direction cannot
exhaust a trader.

`aptos-transaction-workloads-lib` drives this recipe; see `bench_workflows.rs`.
`test_harness_onboard_then_mix` pins it in Move.

## Knobs

Every knob is a runtime argument, so nothing here can be folded away at
compile time.

- **Position count**: `n_positions` in `seed_positions`. Suggested points are
  8, 64 and 512. This is what sets how densely the tick bitmap is populated.
- **Range width**: `width_ticks`. Narrow ranges put more initialized ticks in
  the path of a given price move.
- **Swap distance**: choose `sqrt_price_limit` from
  `clmm_tick_math::get_sqrt_price_at_tick(t)` to cap how far the price travels.
  Roughly 3, 10 and 50 ticks of travel gives three points on the
  crossings-per-swap curve.
- **Swap size and direction**: `amount_in` / `amount_out` and `a_to_b`.
- **Fee rate and tick spacing**: set per pool at `create_pool`. Fee rates are
  millionths, so 3000 is 0.30%. Tick spacing sets both the alignment of usable
  ticks and `max_liquidity_per_tick`.
- **Seed**: `seed` in `seed_positions` drives the house generator
  (`LCG_MUL = 1103515245`, `LCG_INC = 12345`, `LCG_MOD = 1000003`).

A representative mix: seed 64 positions of width 600, then run swaps limited
to 3, 10 and 50 ticks in both directions, a `mint` and `burn` at a fresh
range, and a `collect`.

## Modules

| Module | Contents |
| --- | --- |
| `clmm_full_math` | `mul_div`, `mul_div_rounding_up`, `div_rounding_up`, and the two 256-bit bit scans. |
| `clmm_tick_math` | Tick bounds, `get_sqrt_price_at_tick`, `get_tick_at_sqrt_price`, spacing checks. |
| `clmm_tick_bitmap` | `Table<i16, u256>` bitmap, `compress`, `position`, `flip_tick`, `next_initialized_tick_within_one_word`. |
| `clmm_tick` | `TickInfo`, per-tick update and cross, fee growth inside a range, `max_liquidity_per_tick`. |
| `clmm_liquidity_math` | `add_delta` and the two token-amount identities. |
| `clmm_swap_math` | `compute_swap_step` and the next-price helpers. |
| `clmm_position` | Position keys, fee settlement, `collect`. |
| `clmm_pool` | Pool state, the swap loop, entry points, vaults. |
| `clmm_mock_fa` | Test fungible assets. |

## Numeric conventions

- Prices are `sqrt(price)` in **Q64.96**, held in a `u128`.
- Tick bound is **443636**, matching the on-chain Hyperion interface.
- Fee growth per unit of liquidity is **Q64.64**, held in a `u128` and read
  only as wrapping differences.
- Fee rates are millionths (`FEE_RATE_DENOMINATOR = 1000000`).

The Q64.96 format and the 443636 tick bound come from different places.
Hyperion's deployed interface uses Q64.64 with that bound; Uniswap V3 uses
Q64.96 with a bound of 887272 in a `u160`. This package was specified as Q64.96
with the 443636 bound in a `u128`, and that combination does fit:
`sqrt(1.0001)^443636 * 2^96` is `340275971719517849884101479065584693833`,
about `1.9e-5` below `2^128`.

The cost is a ceiling on liquidity. `get_next_sqrt_price_a_up` forms
`(L << 96) * sqrt_price` in a `u256`, which needs
`L < 2^160 / sqrt_price`. Near tick 0 that allows `L` up to roughly `2^64`; at
tick 100000 it drops to roughly `2^57`. Past that the function aborts with
`EPRICE_OVERFLOW` instead of returning a wrong price. The benchmark's own
liquidity amounts sit around `10^12` to `10^15`, well inside the bound.

## Parity

Parity here means two different things, checked two different ways.

Behavioural parity is agreement with the published Uniswap V3 mathematics, and
it is checked by test. Those checks are listed under "Behaviour" below.

API parity is agreement with Hyperion's deployed interface on names, argument
order and field sets. A Move unit test cannot check this, because there is
nothing in this package to link the deployed modules against. It was checked by
reading the declarations in the decompiled corpus and matching them here. Those
checks are listed under "API contract", each naming the reference file and what
was compared.

### API contract

Read from the decompiled package at
`0x8b4a2c4bb53857c718a04c020b98f8c2e1f99a68b0f57389a8bf5434cd22e05c::dex`.
Declarations only. No bodies were used, and none would port: they are
decompiler output with mangled locals and no named constants.

- **Tick bound 443636.** `tick_math.move` declares `min_tick()` as
  `i32::neg_from(443636u32)` and `max_tick()` as `i32::from(443636u32)`.
  `clmm_tick_math` uses the same bound, as `MAX_TICK: i32 = 443636` and its
  negation. This is a numeric fact about the price grid, not an implementation.
  `test_bounds_agree_with_the_extreme_ticks` pins `min_sqrt_price()` and
  `max_sqrt_price()` to `get_sqrt_price_at_tick` at exactly `±443636`, so the
  bound cannot drift from the derived price table.
- **`compute_swap_step` signature.** `swap_math.move` declares
  `(u128, u128, u128, u64, u64, bool, bool): (u64, u64, u128, u64)`.
  `clmm_swap_math::compute_swap_step` takes the same seven arguments in the
  same order and returns the same four, named
  `(sqrt_current, sqrt_target, liquidity, amount, fee_rate, a_to_b, exact_in)`
  returning `(amount_in, amount_out, next_sqrt_price, fee_amount)`.
- **`TickInfo` field set.** `tick.move` declares `TickInfo has copy, drop,
  store` with `liquidity_gross: u128`, `liquidity_net: i128::I128`,
  `fee_growth_outside_a: u128`, `fee_growth_outside_b: u128`, six oracle,
  rewarder and incentive fields, and `initialized: bool`. `clmm_tick::TickInfo`
  keeps the same abilities and the five fields that drive pricing, and drops
  the six that do not, since this package has no oracle or rewarder.
- **Fee denominator.** `swap_math.move` declares `fee_rate_denominator(): u64`
  returning `1000000`, and divides by that literal in the step.
  `clmm_swap_math::FEE_RATE_DENOMINATOR` is `1000000`, so a fee rate of 3000
  means 0.30% in both.

Two deliberate departures from the reference shape:

- Signed integers are Move's native `i32` and `i128`, not the struct-emulated
  `i32::I32` and `i128::I128` the reference carries. See "Future work" below.
- The oracle, rewarder, blacklist and partnership modules are not reproduced.

### Behaviour

- **Tick to price.** The 19 Q128.128 multipliers were derived from
  `sqrt(1.0001)^(-2^i)` with a throwaway Python `decimal` script at 160 digits
  of precision, not copied. The resulting table is checked into
  `test_sqrt_price_at_reference_ticks` and
  `test_sqrt_price_at_more_reference_ticks` at ticks 0, ±1, ±2, ±3, ±100,
  ±1000, ±10000, ±50000, ±100000, ±200000 and ±443636. The same script
  confirmed the fixed-point result equals the exact floor through ±200000 and
  stays within `1e-29` relative out to the bound.
- **Bounds.** `test_bounds_agree_with_the_extreme_ticks` ties
  `min_sqrt_price()` and `max_sqrt_price()` to `get_sqrt_price_at_tick` at
  ±443636, so the constants cannot drift from the table.
- **Monotonicity.** `test_sqrt_price_is_strictly_increasing` checks the price
  rises with the tick, which is what the swap loop's target selection assumes.
- **Price to tick.** `test_tick_at_sqrt_price_round_trip` and
  `..._dense` check `get_tick_at_sqrt_price(get_sqrt_price_at_tick(t)) == t`
  over a spread including negatives and both bounds.
  `test_tick_at_sqrt_price_floors` checks it returns the tick at or below a
  price that sits between two ticks.
- **Bitmap word and bit split.** `test_position_on_negative_ticks` pins the
  split at -1, -2, -255, -256, -257, -512 and -443636, where truncation toward
  zero would put the bit offset out of range.
  `test_position_round_trip` re-derives the tick from the pair over -1000..1000,
  and `test_compress_floors_on_negative_ticks` covers the spacing division.
- **Bitmap search.** `test_next_initialized_finds_a_flipped_tick_downward` and
  `..._upward` check both directions, including that the downward search is
  inclusive at the current tick and the upward search is not.
  `test_next_initialized_reports_none_within_the_word` checks the "nothing here"
  answer returns the word boundary in each direction and on both signs, and
  `test_next_initialized_stops_at_the_word_edge` checks a tick is invisible from
  the neighbouring word.
- **Swap step conservation.** `test_exact_in_partial_fill_consumes_everything`
  and its upward twin check `amount_in + fee == amount` exactly on a partial
  fill. `test_exact_in_full_step_stops_at_the_target` checks
  `amount_in + fee <= amount` when the step reaches the target.
  `test_fee_is_the_stated_fraction_of_gross_input` and the two exact-out tests
  check that netting the fee back out of `amount_in + fee` returns `amount_in`
  to within one unit.
- **Swap step bounds.** `test_step_never_passes_the_target` runs both
  directions and both modes over five input magnitudes and checks the price
  stays between the start and the target.
- **Tick crossing.**
  `test_swap_across_one_tick_moves_liquidity_by_liquidity_net` mints a base
  range plus a range starting at tick 60, swaps up past tick 60 and checks pool
  liquidity rose by exactly that tick's `liquidity_net`, then swaps back down
  and checks it fell by the same amount. That is the sign flip on a downward
  crossing.
- **Mint and burn round trip.** `test_mint_then_full_burn_restores_the_pool`
  checks pool liquidity returns to zero, both ticks are cleared from the tick
  table, and collecting the credited principal empties the vaults to within the
  rounding unit the pool keeps.
- **Fee attribution across a crossing.** `test_fee_growth_inside` checks that
  growth booked while the price sat inside a range stays attributed to it after
  the price crosses out, and that growth accruing afterwards does not.
  `test_cross_flips_the_accumulators` and `test_cross_wraps_the_accumulators`
  check the `global - outside` flip on its own, including across a wrap.
- **Fee accounting.** `test_fees_reach_the_position` checks a position in range
  for two swaps of one billion at 0.30% is owed just under three million of
  each token, and `test_out_of_range_position_earns_nothing` checks a position
  that was never in range earns zero.
  `test_fee_growth_round_trips_through_a_position` checks the growth the pool
  books converts back to the fee it charged.
- **`mul_div` at the boundary.** `test_mul_div_at_u256_boundary` and
  `test_mul_div_rounding_up_at_u256_boundary` exercise `(2^128-1)^2` and
  `2^256-1`. `test_rounding_up_differs_by_one` checks rounding up adds exactly
  one when there is a remainder and nothing when there is not.
- **End to end.** `test_end_to_end` creates the assets, opens a pool, seeds 16
  jittered positions on top of a wide base range, swaps across several ticks in
  both directions in both modes, collects fees, burns the base position and
  collects again.

Two places deliberately diverge from Uniswap V3's reference implementation:

- `get_tick_at_sqrt_price` ends with an exact correction walk over
  `get_sqrt_price_at_tick` rather than using Uniswap's two empirical
  error-margin constants. The result is the same floor, reached without
  reproducing constants from a BUSL-1.1 source.
- Amounts are `u64` rather than `u256`, since Aptos fungible assets are `u64`.
  `compute_swap_step` asserts each returned amount fits.

## Future work: emulated signed integers

This package uses Move's native `i32` and `i128`. A Move package targeting an
older language version, or one comparing native signed integers against the
struct-emulated form most deployed DEXes use, would instead define:

```move
struct I32 has copy, drop, store { bits: u32 }
```

with two's-complement helpers for `add`, `sub`, `neg`, `lt`, `abs` and
`shr`. That variant would be a useful second configuration: same algorithm,
same call graph, but every tick comparison becomes a function call over a
one-field struct instead of a native instruction. It is not built here.

## Tests

```bash
./target/debug/aptos move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_clmm \
  --skip-fetch-latest-git-deps
```

## Provenance

The mathematics is implemented from the published Uniswap V3 whitepaper: the
`sqrt(1.0001)^tick` price grid, the two token-amount identities, the fee
growth accumulator scheme and the tick bitmap. The API is shaped after
Hyperion's on-chain interface — module and function names, the
`compute_swap_step` signature, the `TickInfo` field list, the 443636 tick
bound and the millionths fee denominator.

No code was copied. Uniswap V3 core is BUSL-1.1 and the Hyperion interface
package declares no license, so neither is a permissible source. Numeric
constants that are facts about the specification — tick bounds, fixed-point
scales, fee denominators — are stated as facts. The 19 tick-math multipliers
were derived independently and are pinned by a checked-in reference table
rather than taken from any implementation.
