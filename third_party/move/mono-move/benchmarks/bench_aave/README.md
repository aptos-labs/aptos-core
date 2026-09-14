# bench_aave

A reduced Aave V3 lending market: bit-packed reserve config, scaled-balance
aTokens and debt tokens, a two-slope interest rate model, a mock oracle, and
real fungible-asset vaults.

## What it stresses

- **A read set that grows with the user, not with the configuration.**
  `aave_logic::calculate_user_account_data` walks the caller's config bitmask
  and, for every set bit, reads the reserve-id table, that reserve's object
  resource group, its oracle price, and the caller's scaled aToken and debt
  balances. Two transactions calling the same entry function touch wildly
  different amounts of state depending on how many reserves the caller is in.
  Nothing else in this suite has that shape.
- **`u256` bit manipulation on the hot path.** Every reserve parameter lives in
  one `u256`. Each getter is an `and` with a negated clear mask plus a shift,
  and the flags decode five bits out of one word. The user bitmask packs two
  bits per reserve into a second `u256` and is read once per loop iteration.
- **Wide fixed-point arithmetic.** RAY is 1e27 and WAD is 1e18, so index
  accrual multiplies two 27-decimal numbers inside a `u256` before dividing
  back down. The compounding path runs a third-order Taylor expansion of
  `e^(r*t)`, nesting two `ray_mul`s per call.
- **Half-up rounding in both directions.** `ray_mul` / `ray_div` round half up,
  but minting and burning pick `ray_div_up` or `ray_div_down` depending on
  which side of the book they touch, so the rounding direction is part of the
  control flow rather than a fixed policy.
- **Resource groups and derived object addresses.** Underlyings are real
  fungible assets, not an internal ledger. Every supply, withdraw, borrow,
  repay and liquidation goes through `primary_fungible_store`, which reads an
  object's resource group and computes a derived store address. Reserve state
  sits in the object resource group next to `ObjectCore`, so one group read
  answers config, both indexes, both rates and the two token addresses.
- **Sparse table lookups.** Scaled balances live in
  `Table<address, UserState>` per token, prices in `Table<address, u256>`, user
  configs and reserve ids in tables on the pool.

## Modules

Seven modules here stand in for the upstream packages.

| Module | Upstream |
| --- | --- |
| `bench::aave_math` | `AaveMath::wad_ray_math`, `AaveMath::math_utils` |
| `bench::aave_config` | `AaveConfig::reserve_config`, `AaveConfig::user_config` |
| `bench::aave_oracle` | `AaveOracle::oracle`, reduced to a mock feed |
| `bench::aave_mock_fa` | `AaveMockUnderlyings::mock_underlying_token_factory` |
| `bench::aave_tokens` | `AavePool::token_base`, `AavePool::a_token_factory`, `AavePool::variable_debt_token_factory` |
| `bench::aave_pool` | `AavePool::pool`, `AavePool::pool_logic`, `AavePool::default_reserve_interest_rate_strategy`, and the reserve-creation half of `AavePool::pool_configurator` |
| `bench::aave_logic` | `AavePool::generic_logic`, `AavePool::validation_logic`, `AavePool::supply_logic`, `AavePool::borrow_logic`, `AavePool::liquidation_logic`, `AavePool::flashloan_logic` |

Kept faithful: the fixed-point math and its rounding, the config bit layout and
its clear masks, the scaled-balance token model, the two-slope rate model, the
reserve cache and `update_state`, the health-factor loop, and the liquidation
bonus, close factor and protocol fee arithmetic.

Dropped outright:

- **Rewards.** `rewards_controller`, `rewards_distributor`, `emission_manager`,
  `transfer_strategy`.
- **E-mode.** `emode_logic`, and the e-mode branches inside the validations and
  the health-factor loop.
- **Isolation mode.** `isolation_mode_logic`, the debt ceiling, and the
  siloed-borrowing checks.
- **UI providers.** `ui_pool_data_provider_v3`,
  `ui_incentive_data_provider_v3`, `pool_data_provider`, and the whole
  `AaveData` package.
- **Collector.** `collector` and `pool_fee_manager`. The reserve factor is
  still accrued into `ReserveData::accrued_to_treasury`, but nothing mints or
  withdraws it.
- **Access control.** `AaveAcl::acl_manage`. Admin functions check
  `signer::address_of(admin) == @bench` directly.
- **Coin migration.** `coin_migrator`, `fungible_asset_manager`,
  `AaveWrapper`.
- **Everything else.** `events`, `pool_token_logic`, `user_logic`, the
  Chainlink adapter and staleness checks in the oracle, and
  `AaveConfig::error_config` / `helper` / `revision`.

Three smaller reductions, so nobody is surprised by a number:

- The dust guard on partial liquidation (`MIN_LEFTOVER_BASE`) is gone.
  Upstream refuses a partial close that would leave a position below roughly
  `2.5e20` base units, which makes every partial liquidation at benchmark
  scale abort. Without it, `liquidation_call` works at any position size.
- The bad-debt deficit path is gone. `liquidation_call` burns exactly the debt
  the liquidator repaid, never more.
- The liquidation protocol fee is paid in aTokens to `@bench` rather than to a
  treasury module, and aToken primary stores are not frozen, because
  `primary_fungible_store::mint` aborts on a frozen store.

### Layering note

The spec put the user entry points in `aave_pool` and
`calculate_user_account_data` in `aave_logic`. That is a module cycle, which
Move rejects: the health-factor loop reads reserve state, and every entry point
calls a validation that calls the health-factor loop. The split here is the one
upstream Aave uses. `aave_pool` is the state layer and holds the admin entry
points; `aave_logic` sits above it and holds the seven user entry points.

## Initialization

Named address `bench` is `0xB0`, and `admin` must be that address.

1. `aave_pool::initialize(admin)` — creates the pool object, the reserve
   registry and the price feed.

2. `aave_mock_fa::create_asset(admin, symbol, decimals)`, N times. Assets are
   named objects seeded by their symbol, so
   `aave_mock_fa::asset_address(symbol)` recovers the address later. Decimals
   must be 6 through 18. `create_asset_entry(admin, symbol, decimals)` is the
   `entry` form.

   ```move
   aave_mock_fa::create_asset(admin, b"USDC", 6);
   let usdc = aave_mock_fa::asset_address(b"USDC");
   ```

3. `aave_pool::admin_add_reserve(admin, underlying, ltv, liquidation_threshold,
   liquidation_bonus, reserve_factor, liquidation_protocol_fee,
   optimal_usage_ratio_bps, base_variable_borrow_rate_bps,
   variable_rate_slope1_bps, variable_rate_slope2_bps)`, N times. Creates the
   reserve object, its aToken, its variable debt token and its vault store.
   Every numeric parameter is basis points; decimals are read off the asset. A
   conventional set is `8000, 8500, 10500, 1000, 1000, 8000, 0, 400, 7500`.

4. `aave_pool::oracle_set_price(admin, underlying, price)`, N times. Prices
   carry 8 decimals, as upstream's base currency does, so `$1.00` is
   `100_000_000`.

5. `aave_mock_fa::mint(admin, underlying, user, amount)` to fund users.
   Minting goes into the recipient's primary fungible store. Note the
   `amount` here is `u64`, because that is what the fungible asset framework
   takes; the `aave_logic` entry points take `u256`.

6. Per user: `aave_logic::supply(user, asset, amount)` on K assets,
   `aave_logic::set_user_use_reserve_as_collateral(user, asset, true)` on each,
   then `aave_logic::borrow(user, asset, amount, 2)` on one or two assets to
   land near a target health factor. Supplying does not enable collateral by
   itself, matching upstream, so the explicit call is required or the user has
   no borrowing power. Mode `2` is variable and is the only mode accepted.

7. Advance the clock. Both indexes are written only when
   `timestamp::now_microseconds()` has moved, so with a frozen clock every
   accrual short-circuits and none of the interest math runs. A month is enough
   for the indexes to leave one ray.

   Rates are denominated per microsecond, not per second as upstream does it.
   The benchmark harness advances the block timestamp by one microsecond per
   block, so a second-denominated reserve would accrue once for the whole run
   and every later transaction would take the short-circuit. The annual rate is
   unchanged; only the granularity differs, and the unit tests pin the same ray
   values as before.

### Bulk seeding

Steps 5 and 6 are one call per user per reserve, which at 1024 users and K of
8 is over 17,000 transactions. `seed_users` collapses all of it:

```move
public entry fun seed_users(
    admin: &signer,
    n_users: u64,
    collaterals_per_user: u64,
    supply_amount: u256,
    borrow_amount: u256,
    seed: u64
)
```

Per synthetic user it mints the underlyings, supplies `supply_amount` on
`collaterals_per_user` reserves, enables each as collateral, and borrows
`borrow_amount` on one further reserve. The reserves are a window of the id
space starting where the house LCG lands, so users overlap without every user
landing on the same reserves. It needs `reserves_count > collaterals_per_user`,
or it aborts with `ENOT_ENOUGH_RESERVES` (79).

Every supply runs before any borrow, so a borrow never arrives at a reserve
whose only supplier is a later user. That is not enough on its own: with few
users the LCG will not cover the id space, so supply a liquidity provider
across all reserves before calling `seed_users`.

Synthetic users are objects under the pool, because Aptos has no way to
produce a signer for a plain address outside tests. Two accessors reach them:

- `aave_pool::seeded_user_address(seed, index): address` — pure arithmetic, so
  a harness can compute it before the user exists. This is what
  `liquidation_call` and the views take.
- `aave_pool::seeded_user_signer(admin, seed, index): signer` — admin only.
  Needed to drive `supply`, `withdraw`, `borrow` and `repay` as that user in
  later transactions, since all four take a `&signer`.

`aave_pool::seeded_user_exists(seed, index)` reports whether one has been
created. Reusing a `seed` returns the same addresses, so a second
`seed_users` call tops up the existing users rather than making new ones.

Read back with `aave_logic::user_account_data(user)`, which returns
`(total_collateral_base, total_debt_base, average_ltv,
average_liquidation_threshold, health_factor, has_zero_ltv_collateral)`, or
`aave_logic::health_factor(user)` for the last value alone.

## Entry points

| Function | Effect |
| --- | --- |
| `aave_pool::initialize(admin)` | Create the pool and the price feed. |
| `aave_pool::admin_add_reserve(admin, underlying, ltv, liq_threshold, liq_bonus, reserve_factor, liq_protocol_fee, optimal_usage_bps, base_rate_bps, slope1_bps, slope2_bps)` | Add a reserve with its two tokens and vault. |
| `aave_pool::oracle_set_price(admin, underlying, price)` | Set a price, 8 decimals. |
| `aave_mock_fa::create_asset_entry(admin, symbol, decimals)` | Create an underlying. |
| `aave_mock_fa::mint(admin, asset, to, amount)` | Fund an address. |
| `aave_mock_fa::faucet(asset, to, amount)` | Same, without the admin check. |
| `aave_logic::seed_users(admin, n_users, collaterals_per_user, supply_amount, borrow_amount, seed)` | Build `n_users` supplied-and-borrowed users in one call. |
| `aave_logic::bench_onboard(account, start, collaterals, supply_amount, borrow_amount)` | Fund, supply `collaterals` reserves from `start`, and take one variable borrow, in one transaction. |
| `aave_logic::bench_supply(account, asset, amount)` | Faucet then supply, so a long run cannot exhaust the account. |
| `aave_logic::supply(account, asset, amount)` | Deposit and mint aTokens. |
| `aave_logic::withdraw(account, asset, amount, to)` | Burn aTokens and release underlying. `amount = u256::MAX` withdraws the full balance and clears the collateral bit. |
| `aave_logic::borrow(account, asset, amount, interest_rate_mode)` | Mint variable debt and release underlying. |
| `aave_logic::repay(account, asset, amount, interest_rate_mode)` | Burn debt, capped at what is outstanding. |
| `aave_logic::set_user_use_reserve_as_collateral(account, asset, use_as_collateral)` | Flip one collateral bit. Turning it off revalidates the health factor. |
| `aave_logic::liquidation_call(account, collateral_asset, debt_asset, user, debt_to_cover, receive_a_token)` | Close part of an unhealthy position. |
| `aave_logic::flash_loan_simple(account, asset, amount, premium_bps, receiver_ops)` | Borrow, run the in-package receiver, repay with a premium, in one transaction. |

Views: `aave_logic::user_account_data`, `aave_logic::health_factor`,
`aave_pool::get_normalized_income`, `get_normalized_debt`,
`get_liquidity_index`, `get_variable_borrow_index`,
`get_current_liquidity_rate`, `get_current_variable_borrow_rate`,
`get_virtual_underlying_balance`, `get_accrued_to_treasury`, `get_user_config`,
`reserves_count`, `vault_balance`.

`flash_loan_simple_take` and `flash_loan_simple_repay` are also public.
`take` returns a `FlashLoanReceipt` with no abilities, so the compiler will not
let a caller end the transaction without repaying.

## Transaction harness

`seed_users` builds its users from admin-derived signers, so every transaction
it produces is signed by the admin and they all serialize on one sequence
number. A harness that wants thousands of independent senders uses the
`bench_*` surface instead:

1. Publisher, once: `initialize`, then per reserve `create_asset_entry`,
   `admin_add_reserve`, `oracle_set_price`, and `bench_supply` for the
   bootstrap liquidity every reserve needs before anyone can borrow from it.
2. Each account, once: `bench_onboard(start, collaterals, supply, borrow)`.
   `start` is any function of the address, so the mix can recompute which
   reserves an account holds without tracking anything.
3. Steady state: `bench_supply`, `withdraw`, `borrow`, `repay`,
   `set_user_use_reserve_as_collateral`, `flash_loan_simple`. Every one of
   these succeeds from a plain signer for as long as the run lasts, provided
   the amounts stay small next to what `bench_onboard` supplied.

`aptos-transaction-workloads-lib` drives exactly this recipe; see
`bench_workflows.rs`. `test_harness_onboard_then_mix` pins it in Move.

## Knobs

Every knob is a runtime argument, so nothing here can be folded away at
compile time.

- **N reserves**: how many times steps 2 through 4 run. Suggested points are
  **4, 8 and 16**. This bounds the `calculate_user_account_data` loop.
- **K collateral assets per user**: how many
  `set_user_use_reserve_as_collateral` calls each user makes. Suggested points
  are **1, 2, 4 and 8**, capped at N. This is the real read-set knob: the loop
  visits every reserve index up to `reserves_count`, but only does the reads
  for indexes whose bits are set. Mixing K across users inside one block is the
  interesting case, because it makes two transactions calling the same entry
  function cost visibly different amounts.
- **Number of users**: **16, 128 and 1024**. Each user adds one `user_configs`
  table item plus one `UserState` row per token they touch, so this sets how
  cold the table lookups are. `seed_users` takes this as `n_users`.
- **`seed`** on `seed_users`: picks which reserve window each user gets. Two
  runs at the same seed produce the same users at the same addresses.
- **`amount`**: on every entry point. Utilization decides which slope of the
  rate model runs, and the `optimal_usage_ratio_bps` argument decides where the
  kink sits.
- **Elapsed microseconds**: whether index accrual and treasury accrual run, or
  short-circuit on `last_update_timestamp == now`. Zero, one hour and one month
  are three distinct paths.
- **`premium_bps`** on `flash_loan_simple`: zero skips the index cumulation
  entirely, nonzero runs it.
- **`receiver_ops`** on `flash_loan_simple`: iterations of the receiver's LCG
  loop between take and repay, using the house generator
  (`LCG_MUL = 1103515245`, `LCG_INC = 12345`, `LCG_MOD = 1000003`).
- **`debt_to_cover`** on `liquidation_call`: full versus partial close, and
  whether the collateral-cap branch runs.
- **`receive_a_token`** on `liquidation_call`: `true` transfers aTokens,
  `false` burns them and moves underlying out of the vault. Different write
  sets.
- **Oracle price**: drives the health factor, so it decides who is
  liquidatable.

A representative mix: 8 reserves and 128 users, K drawn from 1, 2, 4 and 8 so
the read set differs transaction to transaction; then `supply`, `withdraw`,
`borrow` and `repay` against the healthy users, one `flash_loan_simple` with a
9 bps premium and 128 receiver ops, and `liquidation_call` against the users
pushed under water by a price drop.

## Pushing a user under water

The health factor is
`wad_div(percent_mul(total_collateral_base, avg_liquidation_threshold),
total_debt_base)`. `liquidation_call` accepts a user only once it drops below
one WAD, `1e18`. Both legs are priced, so dropping the price of an
asset the user has *both* supplied and borrowed does nothing: it scales
numerator and denominator together. The user must borrow a different asset from
the one posted as collateral.

The fixture in `tests/aave_tests.move` does it like this:

1. Two reserves, both at `$1.00`, liquidation threshold `8500`, bonus `10500`.
2. A liquidity provider supplies 1000 units of the debt asset.
3. The user supplies 1000 units of the collateral asset and enables it, then
   borrows 700 units of the debt asset.
   Health factor is `0.85 * 1000 / 700 = 1.214285714285714286`.
4. `aave_pool::oracle_set_price(admin, collateral_asset, 70_000_000)` drops the
   collateral to `$0.70`. Health factor becomes `0.85 * 700 / 700 = 0.85`.
5. `aave_logic::liquidation_call(liquidator, collateral_asset, debt_asset,
   user, 100_000_000, false)` repays `$100` and pays out
   `149_999_999` units of collateral: `$100` at `$0.70` is `142.857142` units,
   and the 105% bonus lifts it to `149.999999`.

To land on a chosen health factor `h`, multiply the collateral price by
`h / hf_before`, provided the dropped asset is the user's only collateral.

Two things to watch when sizing positions:

- The close factor only engages once both the user's collateral and their debt
  in the pair exceed `5e20` base units, which at 8-decimal prices is `$5e12`.
  Below that a liquidator may close the whole position in one call. Above it,
  and with a health factor above `0.95`, only half the debt may be closed.
- Below a health factor of `1 / 1.05`, closing debt at the bonus makes the
  health factor slightly *worse*, not better. That is upstream behaviour, not a
  bug. The test asserts the exact post-liquidation value `0.842916667666666667`
  from a pre-liquidation `0.85`.

## Numeric conventions

- Indexes and rates are RAY, `1e27`. Health factors are WAD, `1e18`.
- LTV, liquidation threshold, liquidation bonus, reserve factor and the
  liquidation protocol fee are basis points out of `10000`. A bonus of `10500`
  means the liquidator gets 105% of what they paid for.
- Rate model parameters are basis points at the boundary and RAY inside;
  `admin_add_reserve` scales by `1e23`.
- Prices carry 8 decimals. Amounts carry the asset's own decimals, and every
  base-currency value is `amount * price / 10^decimals`.
- A year is `31_536_000` seconds, ignoring leap years, matching upstream.

## Tests

```bash
./target/debug/aptos move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_aave \
  --skip-fetch-latest-git-deps
```

16 tests. Every numeric assertion is hand-computed, not recorded from a run:
the 2.5% borrow and 1.25% supply rates the two-slope model produces at half
utilization, the one-year indexes
(`1.0125e27` linear and `1.025315104166666666666666667e27` compounded, against
`e^0.025 = 1.02531512`), the `3.625` health factor for a two-asset user, the
`149_999_999`-unit liquidation payout, and the config round trips at each
field's boundary values including reserve index 0 and 127.

## Provenance

Ported from [aave/aptos-aave-v3](https://github.com/aave/aptos-aave-v3),
reading the published on-chain sources rather than the decompiler output. That
project is Apache-2.0 licensed, the same as this repository, so the code was
ported and adapted directly rather than reimplemented. Every source file that
derives from it names the upstream modules it came from and carries the
upstream copyright and SPDX header:

```move
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0
```

Copyright (c) Aave DAO and contributors.
