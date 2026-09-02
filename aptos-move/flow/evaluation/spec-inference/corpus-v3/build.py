#!/usr/bin/env python3
"""Generate the corpus-v3 package from the private Etna sources.

`aptos-core` is public and Etna is not, so `package/sources/etna` is gitignored
and only this recipe is committed. Everything the targets need is either
extracted here or vendored from `move-stdlib`/`aptos-stdlib`, so the package
declares no dependencies and can be copied anywhere.

Run `--verify` to regenerate in place and fail if any digest moved.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKAGE = HERE / "package"
# The Etna sources are identified by repository and commit, not by a path on
# one machine. `--etna` may still point at an existing checkout, but the commit
# below is what is read: the generator exports that tree object, so a dirty or
# differently-branched checkout cannot change what the corpus is built from.
ETNA_REPO = "https://github.com/aptos-labs/etna.git"
ETNA_COMMIT = "dd23678f980266360e050037fb78317b13753068"
ETNA_SUBDIR = "move"
ETNA_CACHE = HERE / ".etna"
APTOS_CORE = HERE.parents[4]

# ---------------------------------------------------------------------------
# Extraction helpers
# ---------------------------------------------------------------------------


def etna_tree(checkout: Path | None) -> Path:
    """Materialize the pinned Etna commit and return its `move/` directory.

    Reading a working directory would let an unnoticed checkout state change the
    corpus, which is why the manifest used to record whether the tree was dirty.
    Exporting the commit removes the failure mode instead of reporting it: the
    same commit always yields the same bytes, whatever the checkout is doing.
    """
    ETNA_CACHE.mkdir(parents=True, exist_ok=True)
    repository = checkout or (ETNA_CACHE / "repo")
    if checkout is None and not (repository / ".git").exists():
        print(f"cloning {ETNA_REPO} (private; requires access)")
        subprocess.run(
            ["git", "clone", "--filter=blob:none", "--no-checkout", ETNA_REPO, str(repository)],
            check=True,
        )
    have = subprocess.run(
        ["git", "-C", str(repository), "cat-file", "-e", f"{ETNA_COMMIT}^{{commit}}"],
        capture_output=True,
    )
    if have.returncode != 0:
        subprocess.run(
            ["git", "-C", str(repository), "fetch", "--filter=blob:none", "origin", ETNA_COMMIT],
            check=True,
        )
    tree = ETNA_CACHE / "tree"
    if tree.exists():
        shutil.rmtree(tree)
    tree.mkdir(parents=True)
    archive = subprocess.run(
        ["git", "-C", str(repository), "archive", ETNA_COMMIT, ETNA_SUBDIR],
        capture_output=True,
        check=True,
    )
    subprocess.run(["tar", "-x", "-C", str(tree)], input=archive.stdout, check=True)
    move = tree / ETNA_SUBDIR
    if not move.is_dir():
        raise SystemExit(f"pinned commit has no `{ETNA_SUBDIR}` directory: {ETNA_COMMIT}")
    return move


def find_source(root: Path, name: str) -> Path:
    matches = [p for p in root.rglob(name) if p.is_file()]
    if not matches:
        raise SystemExit(f"source not found under {root}: {name}")
    return sorted(matches)[0]


def block(text: str, pattern: str) -> str:
    """Slice a top-level item from its declaration through its matching brace."""
    match = re.search(pattern, text, re.M)
    if not match:
        raise SystemExit(f"item not found: {pattern}")
    start = text.index("{", match.start())
    depth = 0
    for index in range(start, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[match.start() : index + 1]
    raise SystemExit(f"unbalanced braces for {pattern}")


# ---------------------------------------------------------------------------
# Generated modules
# ---------------------------------------------------------------------------

VAULT_HEADER = """// Extracted from Decibel's `vault.move` share-math cluster.
//
// The four target functions are copied unchanged. Only the carrier `Vault`
// enum is reduced: upstream it also holds an `ExtendRef`, two
// `Object<Metadata>` handles, a redemption config and a portfolio, none of
// which these functions read. Dropping them removes the framework dependency
// without altering a single line of the bodies.
//
// The cluster is the point of this corpus. `calculate_redemption_funds_and_fee`
// calls `calculate_unrealized_fees` and `convert_existing_shares_to_asset_amount`,
// and its freedom from underflow at `shares - shares_for_fee` holds only
// because the fee is bounded by the NAV -- a fact established in the callee's
// contract and invisible in the caller's body.
module decibel_vault::vault_share_math {
    use aptos_std::math64;

    const EINVALID_NUM_SHARES: u64 = 10;
    const ECANNOT_CONTRIBUTE_TO_INSOLVENT_VAULT: u64 = 11;
    const BASIS_POINTS_MULTIPLIER: u64 = 10000;
    const ENABLE_CONTRIBUTION_FEE_REDUCTION: bool = false;
"""

VAULT_ITEMS = [
    (r"^\s*enum VaultState\b", None),
    (r"^\s*enum VaultContributionConfig\b", None),
    (r"^\s*enum VaultFeeConfig\b", None),
    (r"^\s*enum VaultFeeState\b", None),
    (None, """    enum Vault has store {
        V1 {
            contribution_config: VaultContributionConfig,
            fee_config: VaultFeeConfig,
            fee_state: VaultFeeState
        }
    }"""),
    (r"^\s*fun contribution_config_is_closed\b", None),
    (r"^\s*fun calculate_unrealized_fees\b", None),
    (r"^\s*fun convert_new_assets_to_share_count\b", None),
    (r"^\s*fun convert_existing_shares_to_asset_amount\b", None),
    (r"^\s*fun calculate_redemption_funds_and_fee\b", None),
]


def build_vault(etna: Path) -> str:
    text = find_source(etna, "vault.move").read_text(encoding="utf-8")
    parts = [VAULT_HEADER]
    for pattern, literal in VAULT_ITEMS:
        parts.append(literal if literal is not None else block(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


def build_pnl(etna: Path) -> str:
    """`calculate_pnl` with the one global config read lifted to a parameter."""
    text = find_source(etna, "backstop_liquidator_profit_tracker.move").read_text(
        encoding="utf-8"
    )
    body = block(text, r"^\s*fun calculate_pnl\b")
    # Upstream takes `market: Object<PerpMarket>` and reads the multiplier from
    # it. Replacing the read with a parameter is the only edit; every
    # arithmetic line is preserved, including the signed truncation.
    body = body.replace(
        "        market: Object<PerpMarket>,\n", ""
    ).replace(
        "        is_long: bool\n", "        size_multiplier: u64,\n        is_long: bool\n"
    )
    body = re.sub(
        r"\n *let size_multiplier = perp_market_config::get_size_multiplier\(market\);\n",
        "\n",
        body,
    )
    if "perp_market_config" in body or "size_multiplier: u64" not in body:
        raise SystemExit("calculate_pnl extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `backstop_liquidator_profit_tracker.move`.
//
// Upstream reads the size multiplier from a global config
// (`perp_market_config::get_size_multiplier(market)`); here it is a parameter,
// which is the only change. Every arithmetic line is preserved.
//
// The specification interest is signed truncation. `pnl_magnitude` is an
// `i128` that may be negative, and Move's `/` truncates toward zero rather
// than flooring, so the result is not `floor(a/b)` for a negative numerator.
// Negating the result overflows when it is exactly `MIN_I64`. The source
// carries a TODO admitting this rounding disagrees with the position-side
// helpers.
module decibel_dex::extracted_pnl_math {

"""
        + body
        + "\n}\n"
    )


def build_order_validation(etna: Path) -> str:
    """`validate_order_input` with its five global config reads lifted."""
    text = find_source(etna, "spot_engine.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun validate_order_input\b")
    replacements = [
        ("get_min_size(market_addr)", "min_size"),
        ("spot_market_config::get_lot_size(market_addr)", "lot_size"),
        ("spot_market_config::get_tick_size(market_addr)", "tick_size"),
        ("spot_market_config::get_min_price(market_addr)", "min_price"),
        ("spot_market_config::get_max_price(market_addr)", "max_price"),
    ]
    for old, new in replacements:
        if old not in body:
            raise SystemExit(f"validate_order_input extraction missed: {old}")
        body = body.replace(old, new)
    body = body.replace(
        "fun validate_order_input(market_addr: address, size: u64, price: u64): bool {",
        "fun validate_order_input(\n"
        "        size: u64,\n"
        "        price: u64,\n"
        "        min_size: u64,\n"
        "        lot_size: u64,\n"
        "        tick_size: u64,\n"
        "        min_price: u64,\n"
        "        max_price: u64\n"
        "    ): bool {",
    )
    return (
        """// Extracted from Decibel's `spot_engine.move`.
//
// Upstream reads all five bounds from global market config; here they are
// parameters. The conjunction is otherwise copied unchanged.
//
// The specification interest is why this never aborts. Both `%` operations
// divide by a configured value, so the function is abort-free only when
// `lot_size` and `tick_size` are non-zero -- an invariant established at
// market registration and invisible here. A contract claiming `aborts_if
// false` without a matching precondition is wrong.
module decibel_dex::extracted_order_validation {

"""
        + body
        + "\n}\n"
    )


TIER_HEADER = """// Extracted from Decibel's `user_credits.move` tier lookup.
//
// Both functions are copied unchanged; only the `TierConfig` carrier is reduced
// to the `tiers` vector they read, dropping the config version.
//
// The specification interest is that neither loop breaks. Each keeps
// overwriting on a match, so the contract is about the **last** qualifying
// index, not the first -- an invariant over the prefix already scanned, and the
// natural "first match" reading states a different function.
module decibel_campaign::extracted_tier_lookup {
"""

TIER_ITEMS = [
    r"^\s*enum Tier\b",
    None,
    r"^\s*package fun credits_for_duration_days\b",
    r"^\s*package fun leverage_for_tier_rank\b",
]

TIER_CONFIG = """    enum TierConfig has store, drop, copy {
        V1 {
            tiers: vector<Tier>
        }
    }"""


def build_tier_lookup(etna: Path) -> str:
    text = find_source(etna, "user_credits.move").read_text(encoding="utf-8")
    parts = [TIER_HEADER]
    for pattern in TIER_ITEMS:
        parts.append(TIER_CONFIG if pattern is None else block(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


TRADING_HEADER = """// Copied from this repository's own `aptos-experimental` trading kernel
// (`trading/order_book/bulk_order_utils.move`). Public source, but unspecified
// on `main`, so it qualifies on spec absence like the rest of the corpus.
//
// Extracted rather than depended upon because the origin declares friends this
// package does not have. Both bodies are copied unchanged.
//
// Two different loop shapes: an adjacent-pair scan with an early return, and a
// bounded search that stops at the first non-crossing level. Neither result is
// a fold, so a `folds_of` invariant does not apply -- each needs a prefix fact.
module aptos_experimental::extracted_bulk_order_utils {
    use std::option::Option;
"""


def build_trading(_etna: Path) -> str:
    source = (
        APTOS_CORE
        / "aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move"
    )
    text = source.read_text(encoding="utf-8")
    parts = [TRADING_HEADER]
    for pattern in (
        r"^\s*fun validate_price_ordering\b",
        r"^\s*fun discard_price_crossing_levels\b",
    ):
        parts.append(block(text, pattern))
    return "\n\n".join(parts) + "\n}\n"



def build_quote_math(etna: Path) -> str:
    """`compute_quote_needed` with the market-config multiplier lifted."""
    text = find_source(etna, "spot_clearinghouse.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*package fun compute_quote_needed\b")
    body = body.replace(
        "        market_addr: address, bid_prices: &vector<u64>, bid_sizes: &vector<u64>\n",
        "        bid_prices: &vector<u64>, bid_sizes: &vector<u64>, multiplier: u128\n",
    ).replace("package fun", "fun")
    body = re.sub(
        r"\n *let multiplier = base_size_multiplier\(market_addr\);\n", "\n", body
    )
    if "base_size_multiplier" in body or "multiplier: u128" not in body:
        raise SystemExit("compute_quote_needed extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `spot_clearinghouse.move`.
//
// Upstream reads the multiplier from market config; here it is a parameter.
// The loop body and the overflow assert are copied unchanged.
//
// The specification interest is that this floors each level *independently*
// before summing, so the result is `sum(floor(p_i * s_i / m))` and not
// `floor(sum(p_i * s_i) / m)`. The two differ whenever any level has a
// remainder, and the source comment says the per-level form is deliberate so
// bulk withholding reconciles with per-order escrow. A loop invariant that
// accumulates the un-floored product and divides once at the end states a
// different function.
module decibel_dex::extracted_quote_math {

    const MAX_U64: u128 = 0xFFFFFFFFFFFFFFFF;
    const E_QUOTE_BALANCE_OVERFLOW: u64 = 3;

"""
        + body
        + "\n}\n"
    )


def build_base_math(etna: Path) -> str:
    """`compute_base_needed`, copied unchanged: it reads no configuration."""
    text = find_source(etna, "spot_clearinghouse.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*package fun compute_base_needed\b").replace(
        "package fun", "fun"
    )
    if "ask_sizes" not in body or "MAX_U64" not in body:
        raise SystemExit("compute_base_needed extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `spot_clearinghouse.move`.
//
// The body is copied unchanged; unlike its sibling `compute_quote_needed` it
// reads no market configuration, so nothing had to be lifted into a parameter.
//
// The specification interest is that the contract cannot be stated from an
// invariant alone. Naming the accumulated value needs a recursive spec
// function, and ruling out a `u128` overflow of the accumulator before the
// assert needs "every prefix is bounded by the whole" -- monotonicity of a
// running sum, which is induction the solver does not perform and which has to
// be supplied as a lemma. The accumulation is deliberately linear: the
// reasoning is the difficulty, not the solver time.
module decibel_dex::extracted_base_math {

    const MAX_U64: u128 = 0xFFFFFFFFFFFFFFFF;
    const E_BASE_BALANCE_OVERFLOW: u64 = 2;

"""
        + body
        + "\n}\n"
    )


def build_minmax(etna: Path) -> str:
    """`find_min_value` and `find_max_value`, copied unchanged.

    They are taken as a pair on purpose: identical loop shape, opposite
    empty-vector behaviour. A specification that does not distinguish them is
    wrong about one of them.
    """
    text = find_source(etna, "trading_fees_manager.move").read_text(encoding="utf-8")
    bodies = [
        block(text, r"^\s*fun find_min_value\b"),
        block(text, r"^\s*fun find_max_value\b"),
    ]
    return (
        """// Extracted from Decibel's `trading_fees_manager.move`.
//
// Both bodies are copied unchanged. They are a matched pair: the same loop
// shape with opposite empty-vector behaviour -- `find_min_value` reads
// `values[0]` before the loop and so aborts on an empty vector, while
// `find_max_value` starts its accumulator at zero and returns it. A contract
// that states one of them states the other wrongly.
module decibel_dex::extracted_minmax {

"""
        + "\n\n".join(bodies)
        + "\n}\n"
    )


def build_median(etna: Path) -> str:
    """`get_median_price`, copied unchanged: a total function over three values."""
    text = find_source(etna, "price_management.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun get_median_price\b")
    return (
        """// Extracted from Decibel's `price_management.move`.
//
// The body is copied unchanged. It is a deliberate floor case: nested
// comparisons, no loop, no arithmetic and no abort, so the whole contract is
// the median relation itself. It is one of the corpus's `guessable` controls,
// which tell an apparatus failure apart from a genuinely difficult task.
module decibel_dex::extracted_median {

"""
        + body
        + "\n}\n"
    )


def build_bucket_index(etna: Path) -> str:
    """`get_bucket_index` with its carrier reduced to the field it reads."""
    text = find_source(etna, "adl_tracker.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun get_bucket_index\b")
    if "cutoffs" not in body:
        raise SystemExit("get_bucket_index extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `adl_tracker.move`.
//
// The body is copied unchanged; the `LeverageBuckets` carrier is reduced to the
// `cutoffs` vector it reads, dropping a `vector<BigOrderedMap<..>>` the
// function never touches and which would pull in the framework.
//
// The specification interest is the early `return`: the result is the *least*
// index whose cutoff admits the leverage, and `n` when none does. A contract
// stating only "some index whose cutoff admits it" is satisfied by a different
// function.
module decibel_dex::extracted_bucket_index {

    enum LeverageBuckets has store, drop {
        V1 {
            cutoffs: vector<u8>
        }
    }

"""
        + body
        + "\n}\n"
    )


def build_taker_fee(etna: Path) -> str:
    """`calculate_min_net_taker_fee`, copied unchanged."""
    text = find_source(etna, "trading_fees_manager.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun calculate_min_net_taker_fee\b")
    return (
        """// Extracted from Decibel's `trading_fees_manager.move`.
//
// The body is copied unchanged. Two chained percentage reductions in `u128`:
// the specification interest is that each `100 - pct` underflows when the
// percentage exceeds 100, so the function aborts on inputs the source never
// guards against and never mentions.
module decibel_dex::extracted_taker_fee {

"""
        + body
        + "\n}\n"
    )


def build_deviation(etna: Path) -> str:
    """`calculate_deviation_bps` with its `MAX_U64` sentinel declared locally."""
    text = find_source(etna, "oracle.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun calculate_deviation_bps\b")
    if "MAX_U64" not in body:
        raise SystemExit("calculate_deviation_bps extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `oracle.move`.
//
// The body is copied unchanged; `MAX_U64` is declared here because the origin
// module takes it from elsewhere in the package.
//
// The specification interest is the sentinel: a zero price returns the maximum
// value rather than aborting, so a contract that treats the degenerate input as
// an abort is wrong. The ratio itself can still overflow the `u64` result.
module decibel_dex::extracted_deviation {
    use std::math64;

    const MAX_U64: u64 = 18446744073709551615;

"""
        + body
        + "\n}\n"
    )


def build_trial_size(etna: Path) -> str:
    """`trial_size_for`, copied unchanged with the constants it reads."""
    text = find_source(etna, "protected_trial.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*package fun trial_size_for\b").replace("package fun", "fun")
    return (
        """// Extracted from Decibel's `protected_trial.move`.
//
// The body and both asserts are copied unchanged; the constants it reads are
// declared here.
//
// The specification interest is a four-way `u128` product over a product
// divisor, truncating rather than rounding, with an explicit range check on the
// way back to `u64` -- so the contract has two distinct named aborts on top of
// the exact truncating ratio.
module decibel_dex::extracted_trial_size {
    use std::error;

    const BPS_DENOM: u64 = 10_000;
    const EORACLE_STALE_OR_ZERO_MARK: u64 = 10;
    const ESIZE_OVERFLOW_U64: u64 = 14;

"""
        + body
        + "\n}\n"
    )


def build_tier_leverage(etna: Path) -> str:
    """`checked_max_tier_leverage`, copied unchanged."""
    text = find_source(etna, "funded_first_trade.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*fun checked_max_tier_leverage\b")
    return (
        """// Extracted from Decibel's `funded_first_trade.move`.
//
// The body is copied unchanged; the error constant it raises is declared here.
//
// The specification interest is that the abort is universally quantified over
// the vector while the result is an accumulated maximum: the contract needs a
// quantifier for when it fails and a recursive maximum for what it returns, and
// neither states the other.
module decibel_campaign::extracted_tier_leverage {
    use std::error;

    const EFFT_TIER_LEVERAGE_EXCEEDS_MARKET_MAX: u64 = 40;

"""
        + body
        + "\n}\n"
    )


def build_liquidation_price(etna: Path) -> str:
    """`get_liquidation_price` with its carrier reduced to the fields it reads."""
    text = find_source(etna, "liquidation_config.move").read_text(encoding="utf-8")
    body = block(text, r"^\s*package fun get_liquidation_price\b").replace(
        "package fun", "fun"
    )
    if "backstop_margin_maintenance_divisor" not in body:
        raise SystemExit("get_liquidation_price extraction did not apply cleanly")
    return (
        """// Extracted from Decibel's `liquidation_config.move`.
//
// The body is copied unchanged; the `LiquidationConfig` carrier is reduced to
// the four ratio fields it reads, dropping the backstop liquidator address.
//
// The specification interest is the divisor. Both branches divide by
// `market_max_leverage * divisor`, so the function aborts when the leverage is
// zero -- a case the source never mentions and which is easy to miss when the
// contract is written from the arithmetic alone. Both products can also leave
// `u64` before the division happens.
module decibel_dex::extracted_liquidation_price {

    enum LiquidationConfig has store, drop {
        V1 {
            maintenance_margin_leverage_multiplier: u64,
            maintenance_margin_leverage_divisor: u64,
            backstop_margin_maintenance_multiplier: u64,
            backstop_margin_maintenance_divisor: u64
        }
    }

"""
        + body
        + "\n}\n"
    )


def build_work_units(etna: Path) -> str:
    """Two `work_unit_utils` functions with the carrier reduced to `amount`.

    `consume_work_units` is `inline`, so it is expanded at the call site and
    needs no contract of its own; it is copied so the call site keeps its
    meaning.
    """
    text = find_source(etna, "work_unit_utils.move").read_text(encoding="utf-8")
    bodies = [
        block(text, r"^\s*inline fun consume_work_units\b"),
        block(text, r"^\s*package fun consume_order_match_work_units\b").replace(
            "package fun", "fun"
        ),
        block(text, r"^\s*package fun get_max_order_placement_limit\b").replace(
            "package fun", "fun"
        ),
    ]
    return (
        """// Extracted from Decibel's `work_unit_utils.move`.
//
// The bodies are copied unchanged; the `WorkUnit` carrier is reduced to the
// `amount` field they read, dropping a deprecated variant and a progress flag.
//
// Two different shapes. `consume_order_match_work_units` mutates through a
// `&mut` and saturates to zero rather than underflowing -- but its `u32`
// arithmetic can still overflow twice over, once forming the amount to consume
// and once doubling it. `get_max_order_placement_limit` is a clamped division
// whose floor is one, so it never returns zero however small the budget.
module decibel_dex::extracted_work_units {

    enum WorkUnit has copy, drop {
        V1 {
            amount: u32
        }
    }

    const POSITION_STATUS_WORK_UNITS: u32 = 500;
    const ORDER_MATCH_WORK_UNITS: u32 = 1000;

"""
        + "\n\n".join(bodies)
        + "\n}\n"
    )


SELECTION_MACHINE = """// Authored for this corpus rather than extracted from Etna, which has no
// function-valued code.
//
// A bounded selection loop: draw candidates by applying the continuation
// `next`, accept the first admissible one, and after `rounds` unsuccessful
// draws hand back the position to restart from. Both the continuation and the
// admissibility test arrive as function values, so the contract cannot be
// stated without reasoning about them -- what they return, and whether they
// abort. The state after `k` draws is `next` applied `k` times, which needs a
// recursive specification function over a function value.
module decibel_dex::selection_machine {

    enum Outcome has drop {
        Accepted { value: u64, draws: u64 },
        Exhausted { restart_from: u64 }
    }

    fun select(
        start: u64,
        rounds: u64,
        next: |u64| u64 has copy + drop,
        admissible: |u64| bool has copy + drop
    ): Outcome {
        let value = start;
        let i = 0;
        while (i < rounds) {
            if (admissible(value)) {
                return Outcome::Accepted { value, draws: i }
            };
            value = next(value);
            i += 1;
        };
        Outcome::Exhausted { restart_from: value }
    }
}
"""


def build_selection_machine(_etna: Path) -> str:
    """A function-valued state machine, authored rather than extracted.

    Etna has no function-valued code, so without this the benchmark cannot
    exercise contracts over function values at all -- no `result_of`, no
    `aborts_of`, and no continuation passed as data. Because it is ours it
    contains no proprietary source, and it is emitted here rather than committed
    so that every module in the package is produced the same way.
    """
    return SELECTION_MACHINE


LOMUTO_PARTITION = """// Authored for this corpus rather than extracted from Etna, which has no
// sorting code.
//
// The partition step of quicksort, Lomuto style, over a `u64` vector: move
// the chosen pivot to the end, sweep the rest once, swapping every element
// below the pivot value into a growing prefix, and finally swap the pivot in
// behind that prefix. The pivot's value has to be followed through two swaps
// to say where it ends up, and the loop invariant has to hold the prefix, the
// scanned suffix and the parked pivot at once. The only aborts are an empty
// vector and an out-of-range pivot, and the source states neither.
module decibel_dex::lomuto_partition {

    fun partition(values: &mut vector<u64>, pivot: u64): u64 {
        let last = values.length() - 1;
        values.swap(pivot, last);
        let p = values[last];
        let store = 0;
        let i = 0;
        while (i < last) {
            if (values[i] < p) {
                values.swap(i, store);
                store += 1;
            };
            i += 1;
        };
        values.swap(store, last);
        store
    }
}
"""


def build_lomuto_partition(_etna: Path) -> str:
    """Quicksort's partition step, authored rather than extracted.

    Etna has no sorting code, and without this the benchmark has no target
    that rearranges a vector in place: every other loop reads its input or
    accumulates a scalar. Like `selection_machine` it is ours, so it contains
    no proprietary source, and it is emitted here so that every module in the
    package is produced the same way.
    """
    return LOMUTO_PARTITION


GENERATORS = {
    "etna/vault_share_math.move": build_vault,
    "etna/extracted_pnl_math.move": build_pnl,
    "etna/extracted_order_validation.move": build_order_validation,
    "etna/extracted_quote_math.move": build_quote_math,
    "etna/extracted_base_math.move": build_base_math,
    "etna/extracted_minmax.move": build_minmax,
    "etna/extracted_median.move": build_median,
    "etna/extracted_bucket_index.move": build_bucket_index,
    "etna/extracted_taker_fee.move": build_taker_fee,
    "etna/extracted_deviation.move": build_deviation,
    "etna/extracted_trial_size.move": build_trial_size,
    "etna/extracted_tier_leverage.move": build_tier_leverage,
    "etna/extracted_liquidation_price.move": build_liquidation_price,
    "etna/selection_machine.move": build_selection_machine,
    "etna/lomuto_partition.move": build_lomuto_partition,
    "etna/extracted_work_units.move": build_work_units,
    "etna/extracted_tier_lookup.move": build_tier_lookup,
    "trading/extracted_bulk_order_utils.move": build_trading,
}

# ---------------------------------------------------------------------------
# Vendored dependencies
# ---------------------------------------------------------------------------

VENDORED_STDLIB_SKIP = {"reflect.move", "reflect.spec.move", "unit_test.move"}
VENDORED_APTOS_STD = ["math64.move", "math128.move", "fixed_point64.move"]


def vendor_deps(sources: Path) -> dict[str, str]:
    """Copy the stdlib subset the targets need, so the package has no deps."""
    deps = sources / "deps"
    deps.mkdir(parents=True, exist_ok=True)
    written: dict[str, str] = {}
    stdlib = APTOS_CORE / "aptos-move/framework/move-stdlib/sources"
    for path in sorted(stdlib.rglob("*.move")):
        if path.name in VENDORED_STDLIB_SKIP:
            continue
        target = deps / path.name
        target.write_text(path.read_text(encoding="utf-8"), encoding="utf-8")
        written[f"deps/{path.name}"] = target.read_text(encoding="utf-8")
    aptos_std = APTOS_CORE / "aptos-move/framework/aptos-stdlib/sources"
    for name in VENDORED_APTOS_STD:
        path = find_source(aptos_std, name)
        target = deps / name
        target.write_text(path.read_text(encoding="utf-8"), encoding="utf-8")
        written[f"deps/{name}"] = target.read_text(encoding="utf-8")
    return written


# ---------------------------------------------------------------------------
# Provenance
# ---------------------------------------------------------------------------


def repo_identity(repository: Path) -> dict[str, object]:
    def git(*args: str) -> str:
        return subprocess.run(
            ["git", *args], cwd=repository, capture_output=True, text=True, check=True
        ).stdout.strip()

    return {
        # No local path: it is the one field whose value depends on whose
        # machine ran the build, so recording it dirties the manifest for
        # everyone who regenerates and describes nothing about the corpus.
        "commit": git("rev-parse", "HEAD"),
        # An untracked file elsewhere cannot change what is copied, so only
        # tracked modifications count as drift.
        "tracked_modifications": bool(git("status", "--porcelain", "--untracked-files=no")),
    }


def _drift(
    name: str,
    left: dict[str, str],
    right: dict[str, str],
    left_label: str,
    right_label: str,
) -> str:
    """Describe how one file differs, naming both sides.

    A file missing from one side and a file whose contents changed are
    different problems with different remedies, and the labels say which
    direction the difference runs.
    """
    if name not in right:
        return f"{name} (in {left_label}, missing from {right_label})"
    if name not in left:
        return f"{name} (in {right_label}, missing from {left_label})"
    return f"{name} (contents differ between {left_label} and {right_label})"


def generate_sources(sources: Path, etna: Path) -> None:
    """Write the complete generated source tree into `sources`.

    Every file under a package's `sources/` is produced here -- the generators
    and the vendored stdlib subset -- and the directory is gitignored, so it is
    a build output rather than a checked-in tree. Callers pass an empty
    directory: writing over an existing one leaves behind any file that is no
    longer produced, and a stale file is both proved against by the harness and
    recorded into the manifest by the next regeneration.
    """
    for tree in ("etna", "trading"):
        (sources / tree).mkdir(parents=True, exist_ok=True)
    for relative, generator in GENERATORS.items():
        (sources / relative).write_text(generator(etna), encoding="utf-8")
    vendor_deps(sources)


def digests(sources: Path) -> dict[str, str]:
    out = {}
    for path in sorted(sources.rglob("*.move")):
        rel = path.relative_to(sources).as_posix()
        out[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="build from a modified aptos-core tree, recording the corpus as "
        "not reconstructible from its commit",
    )
    parser.add_argument(
        "--etna",
        type=Path,
        help="an existing Etna checkout to read the pinned commit from; "
        "without it the repository is cloned into corpus-v3/.etna",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="regenerate and fail if any digest differs from the manifest",
    )
    args = parser.parse_args()
    if args.etna is not None and not (args.etna / ".git").exists():
        raise SystemExit(f"not a git checkout: {args.etna}")
    # Etna is exported from a pinned commit, but the vendored stdlib and the
    # trading targets are read from the live aptos-core working tree. A tracked
    # modification therefore enters the corpus under a commit that does not
    # contain it, and the recorded hashes then describe a source nobody can
    # reconstruct. Recording the fact is not enough -- the manifest is supposed
    # to be a reproducibility claim -- so a dirty tree has to be deliberate.
    if not args.verify:
        identity = repo_identity(APTOS_CORE)
        if identity["tracked_modifications"] and not args.allow_dirty:
            raise SystemExit(
                f"aptos-core at {identity['commit']} has tracked modifications, so the "
                "corpus would not match its recorded commit; commit them, or pass "
                "--allow-dirty to record the corpus as unreconstructible"
            )
    etna = etna_tree(args.etna)

    sources = PACKAGE / "sources"
    manifest_path = HERE / "manifest.json"

    # Always build into an empty directory, so what is measured is what a
    # clean build produces rather than what has accumulated in `sources/`.
    with tempfile.TemporaryDirectory(prefix="corpus-v3-build-") as temporary:
        fresh = Path(temporary) / "sources"
        fresh.mkdir(parents=True)
        generate_sources(fresh, etna)
        generated = digests(fresh)

        if args.verify:
            recorded = json.loads(manifest_path.read_text(encoding="utf-8"))[
                "generated_file_sha256"
            ]
            failed = False
            drifted = sorted(
                name
                for name in set(recorded) | set(generated)
                if recorded.get(name) != generated.get(name)
            )
            if drifted:
                print("generated files differ from the manifest:", file=sys.stderr)
                for name in drifted:
                    print(
                        f"  {_drift(name, recorded, generated, 'the manifest', 'a clean build')}",
                        file=sys.stderr,
                    )
                failed = True
            # The package on disk is what the prover and the harness actually
            # read, so a tree left over from an earlier build is a real
            # difference even when the generators themselves agree. Verifying
            # must not repair it: overwriting the tree and then hashing the
            # result is how a stale file survives a reproducibility gate.
            if sources.is_dir():
                on_disk = digests(sources)
                stale = sorted(
                    name
                    for name in set(on_disk) | set(generated)
                    if on_disk.get(name) != generated.get(name)
                )
                if stale:
                    print(
                        f"the built package at {sources} is not a clean build "
                        "(rerun without --verify to rebuild it):",
                        file=sys.stderr,
                    )
                    for name in stale:
                        print(
                            f"  {_drift(name, generated, on_disk, 'a clean build', 'the built package')}",
                            file=sys.stderr,
                        )
                    failed = True
            else:
                print(f"note: no built package at {sources}; checked the generators only")
            if failed:
                raise SystemExit(1)
            print(f"verified {len(generated)} generated files against the manifest")
            return

        if sources.exists():
            shutil.rmtree(sources)
        shutil.copytree(fresh, sources)

    manifest = json.loads(manifest_path.read_text(encoding="utf-8")) if manifest_path.is_file() else {}
    manifest["schema_version"] = 1
    manifest["corpus"] = "etna-v3"
    # The live mutant set is `mutants/<task>/mutants.json`, which carries the
    # anchors and the completed validation. An older top-level block survived
    # here across regenerations saying `essential: null` for one task -- two
    # records of one fact, and the stale one looked authoritative for sitting
    # in the manifest. Regeneration drops it rather than preserving it.
    manifest.pop("mutants", None)
    aptos_core = repo_identity(APTOS_CORE)
    aptos_core["reconstructible"] = not aptos_core["tracked_modifications"]
    manifest["provenance"] = {
        "etna": {"repository": ETNA_REPO, "commit": ETNA_COMMIT},
        "aptos_core": aptos_core,
    }
    manifest["generated_file_sha256"] = generated
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"generated {len(generated)} files; manifest updated")


if __name__ == "__main__":
    main()
