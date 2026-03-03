#!/usr/bin/env python3
"""
Apply gas/fee multipliers to Aptos gas parameter definitions.

Gas types receive --gas-multiplier:
  Gas, InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte, InternalGasPerTypeNode

Storage fee types receive --storage-fee-multiplier:
  Fee, FeePerByte, FeePerSlot

All other types are left unchanged.

Values that are not pure integer literals (e.g. constants, expressions) are skipped.

Multipliers can be specified as integers (e.g. 2) or fractions (e.g. 1838/10000).

Usage:
    # Apply to all gas schedule files:
    python3 apply_gas_multiplier.py --gas-multiplier 2 --storage-fee-multiplier 4

    # Fractional scaling:
    python3 apply_gas_multiplier.py --gas-multiplier 1838/10000

    # Dry-run to preview changes:
    python3 apply_gas_multiplier.py --gas-multiplier 2 --dry-run

    # Apply to specific files:
    python3 apply_gas_multiplier.py --gas-multiplier 2 path/to/file.rs
"""

import argparse
import re
import sys
from pathlib import Path

GAS_TYPES = {
    "Gas",
    "InternalGas",
    "InternalGasPerAbstractValueUnit",
    "InternalGasPerArg",
    "InternalGasPerByte",
    "InternalGasPerTypeNode",
}

STORAGE_FEE_TYPES = {
    "Fee",
    "FeePerByte",
    "FeePerSlot",
}

DEFAULT_FILES = [
    "src/gas_schedule/aptos_framework.rs",
    "src/gas_schedule/instr.rs",
    "src/gas_schedule/misc.rs",
    "src/gas_schedule/move_stdlib.rs",
    "src/gas_schedule/table.rs",
    "src/gas_schedule/transaction.rs",
]

SCRIPT_DIR = Path(__file__).parent

# Matches gas entries of the form:
#   [name: Type, "on_chain_name", value]
# capturing: (name, type, on_chain_name_field, value)
ENTRY_RE = re.compile(
    r"\[\s*([a-zA-Z][a-zA-Z0-9_]*)\s*:\s*([a-zA-Z][a-zA-Z0-9_]*)\s*,([^\]]*),\s*([0-9][0-9_]*)\s*,?\s*\]",
    re.MULTILINE | re.DOTALL,
)


def parse_multiplier(s):
    """Parse a multiplier string like '2', '3/2', or '1838/10000' into (numerator, denominator)."""
    if "/" in s:
        num, den = s.split("/", 1)
        return int(num), int(den)
    return int(s), 1


def format_int(value, use_underscores):
    """Format an integer, inserting _ separators every 3 digits when use_underscores is True."""
    s = str(value)
    if not use_underscores or len(s) <= 3:
        return s
    chunks = []
    while s:
        chunks.append(s[-3:])
        s = s[:-3]
    return "_".join(reversed(chunks))


def process_file(filepath, gas_num, gas_den, fee_num, fee_den, dry_run=False):
    """
    Apply multipliers to gas entries in filepath and return list of changes.
    Each change is (name, type, old_str, new_str).
    """
    content = Path(filepath).read_text()
    changes = []

    def replace(match):
        name = match.group(1)
        ty = match.group(2)
        val_str = match.group(4)

        if ty in GAS_TYPES:
            num, den = gas_num, gas_den
        elif ty in STORAGE_FEE_TYPES:
            num, den = fee_num, fee_den
        else:
            return match.group(0)

        old_val = int(val_str.replace("_", ""))
        new_val = (old_val * num) // den

        if new_val == old_val:
            return match.group(0)

        new_val_str = format_int(new_val, use_underscores="_" in val_str)
        changes.append((name, ty, val_str, new_val_str))

        # Replace only the value, preserving surrounding whitespace and formatting.
        start = match.start(4) - match.start()
        end = match.end(4) - match.start()
        return match.group(0)[:start] + new_val_str + match.group(0)[end:]

    new_content = ENTRY_RE.sub(replace, content)

    if changes and not dry_run:
        Path(filepath).write_text(new_content)

    return changes


def main():
    parser = argparse.ArgumentParser(
        description="Apply gas/fee multipliers to Aptos gas parameter definitions.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--gas-multiplier",
        default="1",
        metavar="N[/D]",
        help="Multiplier for gas types, as integer or fraction (default: 1)",
    )
    parser.add_argument(
        "--storage-fee-multiplier",
        default="1",
        metavar="N[/D]",
        help="Multiplier for storage fee types, as integer or fraction (default: 1)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would change without modifying any files.",
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Files to process (default: all gas schedule files).",
    )
    args = parser.parse_args()

    gas_num, gas_den = parse_multiplier(args.gas_multiplier)
    fee_num, fee_den = parse_multiplier(args.storage_fee_multiplier)

    files = args.files if args.files else [SCRIPT_DIR / f for f in DEFAULT_FILES]

    total_params = 0
    total_files = 0

    for filepath in files:
        path = Path(filepath)
        if not path.exists():
            print(f"WARNING: {filepath} not found -- skipping", file=sys.stderr)
            continue

        prefix = "[DRY RUN] " if args.dry_run else ""
        print(f"\n{prefix}Processing {filepath} ...")

        results = process_file(filepath, gas_num, gas_den, fee_num, fee_den, args.dry_run)

        if results:
            for name, type_, old, new in results:
                print(f"  {name} ({type_}): {old} -> {new}")
            total_params += len(results)
            total_files += 1
        else:
            print("  (no changes)")

    action = "Would change" if args.dry_run else "Changed"
    print(f"\n{action} {total_params} parameter(s) across {total_files} file(s).")


if __name__ == "__main__":
    main()
