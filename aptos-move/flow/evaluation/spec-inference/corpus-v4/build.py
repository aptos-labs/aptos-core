#!/usr/bin/env python3
"""Generate the corpus-v4 package from public aptos-core sources.

Every target is either extracted from this repository's `aptos-experimental`
and `aptos-trading` packages or authored for the corpus, and the standard
library subset the targets need is vendored, so the package declares no
dependencies and can be copied anywhere.

Upstream files are read from a pinned aptos-core commit with `git show`, not
from the working tree, so a checkout on another branch or with local edits
builds the same bytes. `package/sources/` is a build output and gitignored;
the manifest records a digest per generated file.

    python3 corpus-v4/build.py            # regenerate the package and manifest digests
    python3 corpus-v4/build.py --verify   # regenerate in memory and compare, writing nothing
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
APTOS_CORE = HERE.parents[4]
APTOS_CORE_REPOSITORY = "https://github.com/aptos-labs/aptos-core.git"
APTOS_CORE_COMMIT = "ec1e75bb095640a47c061c15ac5c45c96dafb50f"

ORDER_BOOK = "aptos-move/framework/aptos-experimental/sources/trading/order_book"
TRADING_ORDERS = "aptos-move/framework/aptos-trading/sources/orders"
MARKET = "aptos-move/framework/aptos-experimental/sources/trading/market"
FRAMEWORK = "aptos-move/framework/aptos-framework/sources"
MOVE_STDLIB = "aptos-move/framework/move-stdlib/sources"
APTOS_STDLIB = "aptos-move/framework/aptos-stdlib/sources"

# ---------------------------------------------------------------------------
# Reading the pinned commit
# ---------------------------------------------------------------------------


def _git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(APTOS_CORE), *args], capture_output=True, text=True, check=True
    ).stdout


def require_commit() -> None:
    have = subprocess.run(
        ["git", "-C", str(APTOS_CORE), "cat-file", "-e", f"{APTOS_CORE_COMMIT}^{{commit}}"],
        capture_output=True,
    )
    if have.returncode != 0:
        raise SystemExit(
            f"aptos-core commit {APTOS_CORE_COMMIT} is not in this clone; "
            f"fetch it from {APTOS_CORE_REPOSITORY}"
        )


def upstream(path: str) -> str:
    """One file of the pinned commit."""
    return _git("show", f"{APTOS_CORE_COMMIT}:{path}")


def upstream_files(directory: str) -> list[str]:
    """Every file under `directory` in the pinned commit, sorted."""
    listing = _git("ls-tree", "-r", "--name-only", APTOS_CORE_COMMIT, directory)
    return sorted(line for line in listing.splitlines() if line)


# ---------------------------------------------------------------------------
# Extraction helpers
# ---------------------------------------------------------------------------


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


def item(text: str, pattern: str) -> str:
    """`block`, re-indented to one module level whatever whitespace preceded it."""
    return "    " + block(text, pattern).lstrip()


def lines_between(text: str, first: str, last: str) -> str:
    """The lines from the one starting with `first` through the one starting with `last`."""
    lines = text.splitlines()
    starts = [i for i, line in enumerate(lines) if line.strip().startswith(first)]
    ends = [i for i, line in enumerate(lines) if line.strip().startswith(last)]
    if len(starts) != 1 or len(ends) != 1 or ends[0] < starts[0]:
        raise SystemExit(f"cannot delimit lines from `{first}` to `{last}`")
    return "\n".join(lines[starts[0] : ends[0] + 1])


def replace_once(text: str, old: str, new: str, what: str) -> str:
    if text.count(old) != 1:
        raise SystemExit(f"{what}: expected exactly one occurrence to replace")
    return text.replace(old, new)


# ---------------------------------------------------------------------------
# aptos_trading::extracted_bulk_order_types
# ---------------------------------------------------------------------------

BULK_ORDER_TYPES_HEADER = """// Extracted from `aptos-trading` (`sources/orders/bulk_order_types.move`).
//
// The request type and its constructor, copied unchanged. The bulk order
// wrapper, the response types and the accessors are not needed by the targets
// of this corpus and are left out.
module aptos_trading::extracted_bulk_order_types {
"""


def build_bulk_order_types() -> str:
    text = upstream(f"{TRADING_ORDERS}/bulk_order_types.move")
    parts = [BULK_ORDER_TYPES_HEADER.rstrip()]
    for pattern in (
        r"^\s*enum BulkOrderRequest\b",
        r"^\s*public fun new_bulk_order_request\b",
    ):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


# ---------------------------------------------------------------------------
# aptos_experimental::extracted_bulk_order_utils
# ---------------------------------------------------------------------------

BULK_ORDER_UTILS_HEADER = """// Extracted from `aptos-experimental`
// (`sources/trading/order_book/bulk_order_utils.move`).
//
// `new_bulk_order_request_with_sanitization`, `validate_not_zero_sizes`,
// `validate_price_ordering` and `discard_price_crossing_levels` are copied
// unchanged. Upstream, `cancel_at_price_level`,
// `match_order_and_get_next_from_bulk_order` and
// `reinsert_order_into_bulk_order` reach the price and size vectors of one
// side of a bulk order through accessors returning a pair of mutable
// references. Here the two vectors are parameters; the rest of each body is
// unchanged. The matched order passed to `reinsert_order_into_bulk_order` is
// reduced to the two fields the function reads.
module aptos_experimental::extracted_bulk_order_utils {
    use std::option::{Self, Option};
    use aptos_trading::extracted_bulk_order_types::{Self as bulk_order_types, BulkOrderRequest};

    /// The matched order, reduced to the fields read here.
    enum OrderMatchDetails has copy, drop {
        V1 {
            price: u64,
            remaining_size: u64
        }
    }

    fun get_price_from_match_details(self: &OrderMatchDetails): u64 {
        self.price
    }

    fun get_remaining_size_from_match_details(self: &OrderMatchDetails): u64 {
        self.remaining_size
    }
"""

#: The upstream accessor through which a function reaches one side's vectors,
#: and the parameter list that replaces it.
_SIDE_ACCESSOR = (
    "        let (prices, sizes) =\n"
    "            order.get_order_request_mut().get_prices_and_sizes_mut(is_bid);\n"
)


def _lift_vectors(text: str, name: str, upstream_signature: str, lifted_signature: str) -> str:
    """`name` with its side's price and size vectors passed as parameters."""
    body = item(text, rf"^\s*public\(friend\) fun {name}\b")
    body = replace_once(body, upstream_signature + _SIDE_ACCESSOR, lifted_signature, name)
    if "BulkOrder" in body or "get_prices_and_sizes_mut" in body:
        raise SystemExit(f"{name}: extraction did not apply cleanly")
    return body


def _lift_reinsert(text: str) -> str:
    """`reinsert_order_into_bulk_order` with its vectors passed as parameters.

    Upstream selects the side from the matched order; the side's vectors now
    arrive directly, so that selection goes away with the accessor.
    """
    body = item(text, r"^\s*public\(friend\) fun reinsert_order_into_bulk_order\b")
    body = replace_once(
        body,
        "    public(friend) fun reinsert_order_into_bulk_order<M: store + copy + drop>(\n"
        "        order: &mut BulkOrder<M>, other: &OrderMatchDetails<M>\n"
        "    ) {\n"
        "        // Reinsert the order into the bulk order\n"
        "        let (prices, sizes) =\n"
        "            order.get_order_request_mut().get_prices_and_sizes_mut(\n"
        "                other.is_bid_from_match_details()\n"
        "            );\n",
        "    fun reinsert_order_into_bulk_order(\n"
        "        prices: &mut vector<u64>, sizes: &mut vector<u64>, other: &OrderMatchDetails\n"
        "    ) {\n",
        "reinsert_order_into_bulk_order",
    )
    if "BulkOrder" in body or "is_bid_from_match_details" in body:
        raise SystemExit("reinsert_order_into_bulk_order: extraction did not apply cleanly")
    return body


def build_bulk_order_utils() -> str:
    text = upstream(f"{ORDER_BOOK}/bulk_order_utils.move")
    constants = lines_between(
        text,
        "// Error codes for various failure scenarios",
        "const MAX_BULK_ORDER_DEPTH_PER_SIDE",
    )
    parts = [BULK_ORDER_UTILS_HEADER.rstrip(), constants]
    parts.append(item(text, r"^\s*public\(friend\) fun new_bulk_order_request_with_sanitization\b"))
    for pattern in (
        r"^\s*fun validate_not_zero_sizes\b",
        r"^\s*fun validate_price_ordering\b",
        r"^\s*fun discard_price_crossing_levels\b",
    ):
        parts.append(item(text, pattern))
    parts.append(_lift_reinsert(text))
    parts.append(
        _lift_vectors(
            text,
            "match_order_and_get_next_from_bulk_order",
            "    public(friend) fun match_order_and_get_next_from_bulk_order<M: store + copy + drop>(\n"
            "        order: &mut BulkOrder<M>, is_bid: bool, matched_size: u64\n"
            "    ): (Option<u64>, Option<u64>) {\n",
            "    fun match_order_and_get_next_from_bulk_order(\n"
            "        prices: &mut vector<u64>, sizes: &mut vector<u64>, matched_size: u64\n"
            "    ): (Option<u64>, Option<u64>) {\n",
        )
    )
    parts.append(
        _lift_vectors(
            text,
            "cancel_at_price_level",
            "    public(friend) fun cancel_at_price_level<M: store + copy + drop>(\n"
            "        order: &mut BulkOrder<M>, price: u64, is_bid: bool\n"
            "    ): u64 {\n",
            "    fun cancel_at_price_level(\n"
            "        prices: &mut vector<u64>, sizes: &mut vector<u64>, price: u64\n"
            "    ): u64 {\n",
        )
    )
    return "\n\n".join(parts) + "\n}\n"


# ---------------------------------------------------------------------------
# aptos_experimental::extracted_price_time_index
# ---------------------------------------------------------------------------

PRICE_TIME_INDEX_HEADER = """// Extracted from `aptos-experimental`
// (`sources/trading/order_book/price_time_index.move`).
//
// `is_taker_order` is copied unchanged. Upstream the index holds the resting
// orders of both sides in two ordered maps; the function observes it only
// through the best bid and the best ask. Here the index is reduced to the two
// best prices, and the two accessors read them instead of the maps.
module aptos_experimental::extracted_price_time_index {
    use std::option::Option;

    /// The order index, reduced to the best price of each side.
    struct PriceTimeIndex has drop {
        best_bid: Option<u64>,
        best_ask: Option<u64>
    }

    fun best_bid_price(self: &PriceTimeIndex): Option<u64> {
        self.best_bid
    }

    fun best_ask_price(self: &PriceTimeIndex): Option<u64> {
        self.best_ask
    }
"""


def build_price_time_index() -> str:
    text = upstream(f"{ORDER_BOOK}/price_time_index.move")
    parts = [PRICE_TIME_INDEX_HEADER.rstrip()]
    parts.append(item(text, r"^\s*public\(friend\) fun is_taker_order\b"))
    return "\n\n".join(parts) + "\n}\n"


# ---------------------------------------------------------------------------
# aptos_experimental::extracted_dead_mans_switch_tracker
# ---------------------------------------------------------------------------

DEAD_MANS_SWITCH_HEADER = """// Extracted from `aptos-experimental`
// (`sources/trading/market/dead_mans_switch_tracker.move`).
//
// The tracker and keep-alive state types, the two events they emit,
// `disable_keep_alive`, `keep_alive` and `is_order_valid` are copied
// unchanged. The tracker's constructor, its administrative setters and the
// test helpers are left out.
module aptos_experimental::extracted_dead_mans_switch_tracker {
    use std::option::Option;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_framework::event;
"""


def build_dead_mans_switch_tracker() -> str:
    text = upstream(f"{MARKET}/dead_mans_switch_tracker.move")
    parts = [DEAD_MANS_SWITCH_HEADER.rstrip()]
    parts.append(
        "\n".join(
            line for line in text.splitlines() if line.strip().startswith("const E_KEEP_ALIVE")
        )
    )
    for pattern in (
        r"^\s*#\[event\]\s*\n\s*enum KeepAliveUpdateEvent\b",
        r"^\s*#\[event\]\s*\n\s*enum KeepAliveDisabledEvent\b",
        r"^\s*struct KeepAliveState\b",
        r"^\s*struct DeadMansSwitchTracker\b",
        r"^\s*public fun is_order_valid\b",
        r"^\s*fun disable_keep_alive\b",
        r"^\s*public\(friend\) fun keep_alive\b",
    ):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


# ---------------------------------------------------------------------------
# Framework and standard-library targets
# ---------------------------------------------------------------------------


def constants(text: str, names: list[str], what: str) -> str:
    """The declarations of the named constants, in source order."""
    pattern = re.compile(r"^\s*const (" + "|".join(names) + r")\b")
    lines = [line for line in text.splitlines() if pattern.match(line)]
    if len(lines) != len(names):
        raise SystemExit(f"{what}: expected constants {names}, found {len(lines)}")
    return "\n".join(lines)


def extract(header: str, text: str, consts: list[str], patterns: list[str], what: str) -> str:
    parts = [header.rstrip()]
    if consts:
        parts.append(constants(text, consts, what))
    parts.extend(item(text, pattern) for pattern in patterns)
    return "\n\n".join(parts) + "\n}\n"


RATE_LIMITER_HEADER = """// Extracted from `aptos-framework` (`sources/account/rate_limiter.move`).
//
// The whole module apart from its tests, copied unchanged.
module aptos_framework::extracted_rate_limiter {
    use aptos_framework::timestamp;
"""


def build_rate_limiter() -> str:
    return extract(
        RATE_LIMITER_HEADER,
        upstream(f"{FRAMEWORK}/account/rate_limiter.move"),
        [],
        [
            r"^\s*enum RateLimiter\b",
            r"^\s*public fun initialize\b",
            r"^\s*public fun request\b",
            r"^\s*fun refill\b",
        ],
        "rate_limiter",
    )


TRANSACTION_LIMITS_HEADER = """// Extracted from `aptos-framework` (`sources/transaction_limits.move`).
//
// The tier and configuration types, the four functions that build, validate
// and search tier vectors, the governance update of the on-chain
// configuration and the stake check against it, copied unchanged with the
// constants they read. The request types, genesis initialization and the
// stake-pool lookups are left out.
module aptos_framework::extracted_transaction_limits {
    use aptos_framework::system_addresses;
    use std::error;
"""


def build_transaction_limits() -> str:
    return extract(
        TRANSACTION_LIMITS_HEADER,
        upstream(f"{FRAMEWORK}/transaction_limits.move"),
        [
            "ETHRESHOLDS_NOT_MONOTONIC",
            "EINVALID_MULTIPLIER",
            "EMULTIPLIER_NOT_AVAILABLE",
            "EVECTOR_LENGTH_MISMATCH",
            "MIN_MULTIPLIER_PERCENT",
            "MAX_MULTIPLIER_PERCENT",
            "EINSUFFICIENT_STAKE",
        ],
        [
            r"^\s*struct TxnLimitTier\b",
            r"^\s*enum TxnLimitsConfig\b",
            r"^\s*enum RequestedMultipliers\b",
            r"^\s*friend fun new_tier\b",
            r"^\s*fun validate_tiers\b",
            r"^\s*fun new_tiers\b",
            r"^\s*fun find_min_stake_required\b",
            r"^\s*public fun update_config\b",
            r"^\s*fun validate_enough_stake\b",
        ],
        "transaction_limits",
    )


ETHEREUM_HEADER = """// Extracted from `aptos-framework`
// (`sources/account/common_account_abstractions/ethereum_derivable_account.move`).
//
// The scheme check of a sign-in message and the character class it uses,
// copied unchanged with the constants they read.
module aptos_framework::extracted_ethereum_derivable_account {
"""


def build_ethereum_derivable_account() -> str:
    return extract(
        ETHEREUM_HEADER,
        upstream(f"{FRAMEWORK}/account/common_account_abstractions/ethereum_derivable_account.move"),
        ["EINVALID_SCHEME", "MAX_SCHEME_LEN"],
        [r"^\s*fun is_alpha\b", r"^\s*fun validate_scheme\b"],
        "ethereum_derivable_account",
    )


SUI_HEADER = """// Extracted from `aptos-framework`
// (`sources/account/common_account_abstractions/sui_derivable_account.move`).
//
// The signature splitter, copied unchanged with the constant it reads.
module aptos_framework::extracted_sui_derivable_account {
    use std::vector;
"""


def build_sui_derivable_account() -> str:
    return extract(
        SUI_HEADER,
        upstream(f"{FRAMEWORK}/account/common_account_abstractions/sui_derivable_account.move"),
        ["EINVALID_SIGNATURE_LENGTH"],
        [r"^\s*public fun split_signature_bytes\b"],
        "sui_derivable_account",
    )


MULTISIG_HEADER = """// Extracted from `aptos-framework` (`sources/multisig_account.move`).
//
// `get_transaction`, `get_pending_transactions`, `can_execute_with_timelock`
// and `available_transaction_queue_capacity` are copied unchanged, except that
// `can_execute_with_timelock` is declared `fun` instead of `inline fun`: an
// inline function is expanded into its callers and cannot be verified on its
// own. The account resource is reduced to its transaction table and its two
// sequence numbers, and a transaction to its creation time; owners, votes,
// payloads, metadata and event handles are not read here. The timelock
// resource is copied unchanged.
module aptos_framework::extracted_multisig_account {
    use aptos_framework::timestamp::now_seconds;
    use aptos_std::table::{Self, Table};
    use std::error;
    use std::option::Option;

    /// The multisig account, reduced to the fields read here.
    struct MultisigAccount has key {
        transactions: Table<u64, MultisigTransaction>,
        last_executed_sequence_number: u64,
        next_sequence_number: u64,
    }

    /// A multisig transaction, reduced to the field read here.
    struct MultisigTransaction has copy, drop, store {
        creation_time_secs: u64,
    }
"""

MULTISIG_SPEC_HEADER = """// Extracted from `aptos-framework` (`sources/multisig_account.spec.move`): the
// specification of `get_transaction`, copied unchanged.
spec aptos_framework::extracted_multisig_account {
"""


def build_multisig_account() -> str:
    text = upstream(f"{FRAMEWORK}/multisig_account.move")
    parts = [MULTISIG_HEADER.rstrip(), constants(text, ["EINVALID_SEQUENCE_NUMBER", "MAX_PENDING_TRANSACTIONS"], "multisig")]
    parts.append(item(text, r"^\s*enum MultisigAccountTimeLock\b"))
    for pattern in (
        r"^\s*public fun get_transaction\b",
        r"^\s*public fun get_pending_transactions\b",
        r"^\s*public fun available_transaction_queue_capacity\b",
    ):
        parts.append(item(text, pattern))
    timelock = item(text, r"^\s*inline fun can_execute_with_timelock\b")
    parts.append(replace_once(timelock, "    inline fun can_execute_with_timelock(", "    fun can_execute_with_timelock(", "multisig"))
    return "\n\n".join(parts) + "\n}\n"


def build_multisig_account_spec() -> str:
    text = upstream(f"{FRAMEWORK}/multisig_account.spec.move")
    return MULTISIG_SPEC_HEADER.rstrip() + "\n\n" + item(text, r"^\s*spec get_transaction\b") + "\n}\n"


JWKS_HEADER = """// Extracted from `aptos-framework` (`sources/jwks.move`).
//
// `upsert_provider_jwks` is copied unchanged. A provider's entry is reduced to
// its issuer and version; its key set holds `Any`-packed keys, which the
// function only moves.
module aptos_framework::extracted_jwks {
    use std::option;
    use std::option::Option;
    use aptos_std::comparator::compare_u8_vector;

    /// A provider and its version, without its keys.
    struct ProviderJWKs has copy, drop, store {
        issuer: vector<u8>,
        version: u64,
    }
"""


def build_jwks() -> str:
    text = upstream(f"{FRAMEWORK}/jwks.move")
    parts = [JWKS_HEADER.rstrip()]
    parts.append(item(text, r"^\s*struct AllProvidersJWKs\b"))
    parts.append(item(text, r"^\s*fun upsert_provider_jwks\b"))
    return "\n\n".join(parts) + "\n}\n"


VECTOR_RANGE_HEADER = """// Extracted from `move-stdlib` (`sources/vector.move`).
//
// `range_with_step`, copied unchanged with the constant it reads.
module std::extracted_vector_range {
"""


def build_vector_range() -> str:
    return extract(
        VECTOR_RANGE_HEADER,
        upstream(f"{MOVE_STDLIB}/vector.move"),
        ["EINVALID_STEP"],
        [r"^\s*public fun range_with_step\b"],
        "vector",
    )


# ---------------------------------------------------------------------------
# Framework modules reduced to what the targets call
# ---------------------------------------------------------------------------

TIMESTAMP_HEADER = """// Extracted from `aptos-framework` (`sources/timestamp.move`).
//
// The clock resource and its two readers, copied unchanged. Genesis setup,
// the VM update path and the test helpers are left out.
module aptos_framework::timestamp {
"""

TIMESTAMP_SPEC_HEADER = """// Extracted from `aptos-framework` (`sources/timestamp.spec.move`): the
// specifications of the two readers, copied unchanged.
spec aptos_framework::timestamp {
"""


def build_timestamp() -> str:
    text = upstream(f"{FRAMEWORK}/timestamp.move")
    parts = [TIMESTAMP_HEADER.rstrip()]
    parts.append(item(text, r"^\s*struct CurrentTimeMicroseconds\b"))
    parts.append(
        "\n".join(line for line in text.splitlines() if line.strip().startswith("const MICRO_CONVERSION_FACTOR"))
    )
    for pattern in (r"^\s*public fun now_microseconds\b", r"^\s*public fun now_seconds\b"):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


def build_timestamp_spec() -> str:
    text = upstream(f"{FRAMEWORK}/timestamp.spec.move")
    parts = [TIMESTAMP_SPEC_HEADER.rstrip()]
    for pattern in (
        r"^\s*spec now_microseconds\b",
        r"^\s*spec now_seconds\b",
        r"^\s*spec fun spec_now_microseconds\b",
        r"^\s*spec fun spec_now_seconds\b",
    ):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


SYSTEM_ADDRESSES_HEADER = """// Extracted from `aptos-framework` (`sources/system_addresses.move`).
//
// The framework-account check and the predicate it uses, copied unchanged
// with the constant they read. The checks for the other reserved addresses
// are left out.
module aptos_framework::system_addresses {
    use std::error;
    use std::signer;
"""

SYSTEM_ADDRESSES_SPEC_HEADER = """// Extracted from `aptos-framework` (`sources/system_addresses.spec.move`): the
// module pragmas, the specifications of the two functions and the schema they
// include, copied unchanged.
spec aptos_framework::system_addresses {
"""


def build_system_addresses() -> str:
    return extract(
        SYSTEM_ADDRESSES_HEADER,
        upstream(f"{FRAMEWORK}/system_addresses.move"),
        ["ENOT_APTOS_FRAMEWORK_ADDRESS"],
        [r"^\s*public fun assert_aptos_framework\b", r"^\s*public fun is_aptos_framework_address\b"],
        "system_addresses",
    )


def build_system_addresses_spec() -> str:
    text = upstream(f"{FRAMEWORK}/system_addresses.spec.move")
    parts = [SYSTEM_ADDRESSES_SPEC_HEADER.rstrip()]
    for pattern in (
        r"^\s*spec module\b",
        r"^\s*spec assert_aptos_framework\b",
        r"^\s*spec is_aptos_framework_address\b",
        r"^\s*spec schema AbortsIfNotAptosFramework\b",
    ):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


EVENT_HEADER = """// Extracted from `aptos-framework` (`sources/event.move`).
//
// Module events only: `emit` and the native it calls, copied unchanged. Event
// handles and their GUIDs are left out.
module aptos_framework::event {
"""

EVENT_SPEC_HEADER = """// Extracted from `aptos-framework` (`sources/event.spec.move`): the module
// pragmas and the specifications of `emit` and its native, copied unchanged.
spec aptos_framework::event {
"""


def build_event() -> str:
    text = upstream(f"{FRAMEWORK}/event.move")
    parts = [EVENT_HEADER.rstrip()]
    for pattern in (r"^\s*public fun emit\b", r"^\s*native fun write_module_event_to_store\b"):
        match = re.search(pattern, text, re.M)
        if not match:
            raise SystemExit(f"event: item not found: {pattern}")
        if "native" in pattern:
            end = text.index(";", match.start()) + 1
            parts.append("    " + text[match.start() : end].lstrip())
        else:
            parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


def build_event_spec() -> str:
    text = upstream(f"{FRAMEWORK}/event.spec.move")
    parts = [EVENT_SPEC_HEADER.rstrip()]
    for pattern in (
        r"^\s*spec module\b",
        r"^\s*spec emit\b",
        r"^\s*spec write_module_event_to_store\b",
    ):
        parts.append(item(text, pattern))
    return "\n\n".join(parts) + "\n}\n"


# ---------------------------------------------------------------------------
# Authored modules
# ---------------------------------------------------------------------------

SELECTION_MACHINE = """// Authored for this corpus.
//
// A bounded selection loop over function values: draw candidates by applying
// the continuation `next`, accept the first admissible one, and after `rounds`
// unsuccessful draws hand back the position to restart from.
module inference_corpus::selection_machine {

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

LOMUTO_PARTITION = """// Authored for this corpus.
//
// The partition step of quicksort, Lomuto style, over a `u64` vector: move the
// chosen pivot to the end, sweep the rest once, swapping every element below
// the pivot value into a growing prefix, and finally swap the pivot in behind
// that prefix.
module inference_corpus::lomuto_partition {

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

GENERATORS = {
    "trading/extracted_bulk_order_types.move": build_bulk_order_types,
    "trading/extracted_bulk_order_utils.move": build_bulk_order_utils,
    "trading/extracted_price_time_index.move": build_price_time_index,
    "trading/extracted_dead_mans_switch_tracker.move": build_dead_mans_switch_tracker,
    "framework/extracted_rate_limiter.move": build_rate_limiter,
    "framework/extracted_transaction_limits.move": build_transaction_limits,
    "framework/extracted_ethereum_derivable_account.move": build_ethereum_derivable_account,
    "framework/extracted_sui_derivable_account.move": build_sui_derivable_account,
    "framework/extracted_multisig_account.move": build_multisig_account,
    "framework/extracted_multisig_account.spec.move": build_multisig_account_spec,
    "framework/extracted_jwks.move": build_jwks,
    "stdlib/extracted_vector_range.move": build_vector_range,
    "deps/timestamp.move": build_timestamp,
    "deps/timestamp.spec.move": build_timestamp_spec,
    "deps/system_addresses.move": build_system_addresses,
    "deps/system_addresses.spec.move": build_system_addresses_spec,
    "deps/event.move": build_event,
    "deps/event.spec.move": build_event_spec,
    "authored/selection_machine.move": lambda: SELECTION_MACHINE,
    "authored/lomuto_partition.move": lambda: LOMUTO_PARTITION,
}

# ---------------------------------------------------------------------------
# Vendored dependencies
# ---------------------------------------------------------------------------

VENDORED_STDLIB_SKIP = {"reflect.move", "reflect.spec.move", "unit_test.move"}
VENDORED_APTOS_STD = [
    "math64.move",
    "math64.spec.move",
    "table.move",
    "table.spec.move",
    "table_with_length.move",
    "table_with_length.spec.move",
    "data_structures/storage_slots_allocator.move",
    "data_structures/storage_slots_allocator.spec.move",
    "comparator.move",
    "comparator.spec.move",
]
#: The ordered maps, vendored whole: the prover models both as intrinsic maps,
#: so their bodies are a boundary rather than something a target proof reads.
VENDORED_FRAMEWORK = [
    "datastructures/ordered_map.move",
    "datastructures/ordered_map.spec.move",
    "datastructures/big_ordered_map.move",
    "datastructures/big_ordered_map.spec.move",
]


def vendored() -> dict[str, str]:
    """The stdlib subset the targets need, so the package has no dependencies."""
    files: dict[str, str] = {}
    for path in upstream_files(MOVE_STDLIB):
        name = Path(path).name
        if path.endswith(".move") and name not in VENDORED_STDLIB_SKIP:
            files[f"deps/{name}"] = upstream(path)
    for name in VENDORED_APTOS_STD:
        files[f"deps/{Path(name).name}"] = upstream(f"{APTOS_STDLIB}/{name}")
    for name in VENDORED_FRAMEWORK:
        files[f"deps/{Path(name).name}"] = upstream(f"{FRAMEWORK}/{name}")
    return files


# ---------------------------------------------------------------------------
# Building and verifying
# ---------------------------------------------------------------------------


def generate() -> dict[str, str]:
    """Every file of `package/sources/`, by relative path."""
    require_commit()
    files = {relative: generator() for relative, generator in GENERATORS.items()}
    files.update(vendored())
    return files


def digests(files: dict[str, str]) -> dict[str, str]:
    return {
        name: hashlib.sha256(text.encode("utf-8")).hexdigest() for name, text in sorted(files.items())
    }


def on_disk(sources: Path) -> dict[str, str]:
    return {
        path.relative_to(sources).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(sources.rglob("*.move"))
    }


def _differences(left: dict[str, str], right: dict[str, str], left_label: str, right_label: str) -> list[str]:
    out = []
    for name in sorted(set(left) | set(right)):
        if name not in right:
            out.append(f"{name} (in {left_label}, missing from {right_label})")
        elif name not in left:
            out.append(f"{name} (in {right_label}, missing from {left_label})")
        elif left[name] != right[name]:
            out.append(f"{name} (contents differ between {left_label} and {right_label})")
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verify",
        action="store_true",
        help="regenerate and fail if any digest differs from the manifest or the built package",
    )
    args = parser.parse_args()

    sources = PACKAGE / "sources"
    manifest_path = HERE / "manifest.json"
    generated = digests(generate())

    if args.verify:
        recorded = json.loads(manifest_path.read_text(encoding="utf-8"))["generated_file_sha256"]
        failures = _differences(recorded, generated, "the manifest", "a clean build")
        if sources.is_dir():
            failures += _differences(
                generated, digests(on_disk(sources)), "a clean build", "the built package"
            )
        if failures:
            print("corpus-v4 does not reproduce:", file=sys.stderr)
            for line in failures:
                print(f"  {line}", file=sys.stderr)
            raise SystemExit(1)
        print(f"verified {len(generated)} generated files")
        return

    with tempfile.TemporaryDirectory(prefix="corpus-v4-build-") as temporary:
        fresh = Path(temporary) / "sources"
        for relative, text in generate().items():
            path = fresh / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        if sources.exists():
            shutil.rmtree(sources)
        shutil.copytree(fresh, sources)

    manifest = json.loads(manifest_path.read_text(encoding="utf-8")) if manifest_path.is_file() else {}
    manifest["schema_version"] = 1
    manifest["corpus"] = "public-v4"
    manifest["provenance"] = {
        "aptos_core": {"repository": APTOS_CORE_REPOSITORY, "commit": APTOS_CORE_COMMIT}
    }
    manifest["generated_file_sha256"] = generated
    manifest.setdefault("records", [])
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"generated {len(generated)} files; manifest updated")


if __name__ == "__main__":
    main()
