#!/usr/bin/env python3
import sys, csv, re
from collections import defaultdict

HEADER = [
    "Scheme",
    "Setup",
    "Deal (ms)",
    "Serialize (ms)",
    "Aggregate (ms)",
    "Verify (ms)",
    "Decrypt-share (ms)",
]

V1_NAME = "chunky_v1"
V2_NAME = "chunky_v2"

# Match patterns like "pvss/chunky_v1/bls12-381", "pvss_chunky_v1_bls12-381", or just "chunky_v1"
# Criterion converts slashes to underscores in CSV output, so we match both formats
V1_GROUP_PATTERN = re.compile(r"chunky_v1")
V2_GROUP_PATTERN = re.compile(r"chunky_v2")

OPERATIONS = ["deal", "serialize", "aggregate", "verify", "decrypt-share"]


def read_rows(fp):
    reader = csv.DictReader(fp)
    for row in reader:
        for k in row:
            if isinstance(row[k], str):
                row[k] = row[k].strip()
        yield row


def ns_to_ms(ns):
    return round(float(ns) / 1e6, 2)


def fmt_ms(x):
    return f"{x:,.2f}"


def fmt_ratio(r):
    return f"{r:.2f}x"


GREEN = 'color:#15803d'  # bold green when faster
RED = 'color:#dc2626'  # red when slower


def decorate_v2(value_ms, ratio):
    """
    Returns:
      display_text: plain text '1,234.56 (1.20x)' for width calculation
      render_text:  HTML-styled number only; ratio stays uncolored
                    faster (ratio > 1.0): bold green number
                    slower (ratio < 1.0): red number
                    equal  (ratio == 1.0): unstyled number
    """
    num_txt = fmt_ms(value_ms)
    ratio_txt = f" ({fmt_ratio(ratio)})"
    display = num_txt + ratio_txt

    if ratio > 1.0:
        # v2 is faster (ratio > 1.0 means v1 took longer) → bold green number
        render = f"<span style=\"{GREEN}; font-weight:700\">{num_txt}</span>{ratio_txt}"
    elif ratio < 1.0:
        # v2 is slower (ratio < 1.0 means v1 took less time) → red number
        render = f"<span style=\"{RED}\">{num_txt}</span>{ratio_txt}"
    else:
        # equal → no styling
        render = display

    return display, render


def parse_group(group):
    """Parse the Group column to determine if it's v1 or v2."""
    if V1_GROUP_PATTERN.search(group):
        return "v1"
    elif V2_GROUP_PATTERN.search(group):
        return "v2"
    return None


def parse_operation(ident):
    """Extract operation type from Id column like 'deal/...' or 'deal_...' or 'serialize/...'"""
    for op in OPERATIONS:
        if ident.startswith(op + "/") or ident.startswith(op + "_"):
            return op
    return None


def parse_setup(ident, parameter):
    """
    Parse the setup identifier from either the Id column or Parameter column.
    The Id column has format like 'deal/{config_string}', so we extract the config part.
    If Parameter is non-empty, use that; otherwise extract from Id after the operation prefix.
    """
    if parameter and parameter.strip():
        return parameter.strip()
    
    # Extract from Id field: "deal/{config}" -> "{config}"
    for op in OPERATIONS:
        if ident.startswith(op + "/"):
            return ident[len(op) + 1:]  # Remove "op/" prefix
        elif ident.startswith(op + "_"):
            return ident[len(op) + 1:]  # Remove "op_" prefix
    
    # Fallback: use the full ident if we can't parse it
    return ident


def accumulate(rows):
    """
    Build nested dict: setup -> version -> operation -> time_ns
    """
    data = defaultdict(lambda: defaultdict(lambda: defaultdict(float)))

    for r in rows:
        group = r.get("Group", "")
        ident = r.get("Id", "")
        param = r.get("Parameter", "")
        mean_ns = r.get("Mean(ns)", "")

        if mean_ns == "" or mean_ns is None:
            continue
        try:
            mean_ns = float(mean_ns)
        except ValueError:
            continue

        version = parse_group(group)
        if version is None:
            continue

        operation = parse_operation(ident)
        if operation is None:
            continue

        setup = parse_setup(ident, param)
        data[setup][version][operation] = mean_ns

    return data


def make_rows_for_setup(setup, v1_data, v2_data):
    """
    Create rows comparing v1 and v2 for a single setup.
    """
    rows = []

    # Check if we have all operations for both versions
    v1_complete = all(op in v1_data for op in OPERATIONS)
    v2_complete = all(op in v2_data for op in OPERATIONS)

    if not v1_complete and not v2_complete:
        return rows

    # Build row for v1
    if v1_complete:
        v1_row = {
            "Scheme": V1_NAME,
            "Setup": setup,
        }
        for op in OPERATIONS:
            v1_ms = ns_to_ms(v1_data[op])
            v1_row[f"{op}_display"] = fmt_ms(v1_ms)
            v1_row[f"{op}_render"] = fmt_ms(v1_ms)
        rows.append(v1_row)

    # Build row for v2
    if v2_complete:
        v2_row = {
            "Scheme": V2_NAME,
            "Setup": setup,
        }
        for op in OPERATIONS:
            v2_ms = ns_to_ms(v2_data[op])
            if v1_complete and op in v1_data:
                v1_ms = ns_to_ms(v1_data[op])
                ratio = v1_ms / v2_ms if v2_ms > 0 else float("inf")
                disp, rend = decorate_v2(v2_ms, ratio)
                v2_row[f"{op}_display"] = disp
                v2_row[f"{op}_render"] = rend
            else:
                v2_row[f"{op}_display"] = fmt_ms(v2_ms)
                v2_row[f"{op}_render"] = fmt_ms(v2_ms)
        rows.append(v2_row)

    return rows


def padded_table(rows):
    """
    Compute widths from the plain display strings, then emit
    padded Markdown rows with the render strings (HTML-styled).
    """
    cols = HEADER
    display_map = {
        "Scheme": "Scheme",
        "Setup": "Setup",
        "Deal (ms)": "deal_display",
        "Serialize (ms)": "serialize_display",
        "Aggregate (ms)": "aggregate_display",
        "Verify (ms)": "verify_display",
        "Decrypt-share (ms)": "decrypt-share_display",
    }
    render_map = {
        "Scheme": "Scheme",
        "Setup": "Setup",
        "Deal (ms)": "deal_render",
        "Serialize (ms)": "serialize_render",
        "Aggregate (ms)": "aggregate_render",
        "Verify (ms)": "verify_render",
        "Decrypt-share (ms)": "decrypt-share_render",
    }

    widths = {c: len(c) for c in cols}
    for r in rows:
        for c in cols:
            widths[c] = max(widths[c], len(str(r.get(display_map[c], ""))))

    right_cols = {
        "Deal (ms)",
        "Serialize (ms)",
        "Aggregate (ms)",
        "Verify (ms)",
        "Decrypt-share (ms)",
    }

    def pad(c, s, align):
        s = str(s)
        if align == "right":
            return " " + s.rjust(widths[c]) + " "
        return " " + s.ljust(widths[c]) + " "

    header_line = "|" + "|".join(pad(c, c, "left") for c in cols) + "|"
    sep_line = "|" + "|".join("-" * (widths[c] + 2) for c in cols) + "|"

    body_lines = []
    for r in rows:
        cells = []
        for c in cols:
            align = "right" if c in right_cols else "left"
            # use display to compute width, render for content
            content = r.get(render_map[c], "")
            cells.append(pad(c, content, align))
        body_lines.append("|" + "|".join(cells) + "|")

    return "\n".join([header_line, sep_line] + body_lines)


def main():
    # Read CSV from file or stdin
    if len(sys.argv) > 1 and sys.argv[1] != "-":
        with open(sys.argv[1], newline="") as f:
            rows = list(read_rows(f))
    else:
        rows = list(read_rows(sys.stdin))

    data = accumulate(rows)

    if not data:
        print("No PVSS benchmark data found!", file=sys.stderr)
        sys.exit(1)

    # Generate a separate table for each setup
    setups = sorted(data.keys())
    tables = []
    
    for setup in setups:
        v1_data = data[setup].get("v1", {})
        v2_data = data[setup].get("v2", {})
        tbl_rows = make_rows_for_setup(setup, v1_data, v2_data)
        
        if tbl_rows:
            tables.append(padded_table(tbl_rows))
    
    if not tables:
        print("No complete benchmark data found!", file=sys.stderr)
        sys.exit(1)

    # Print all tables separated by double newlines
    print("\n\n".join(tables))


if __name__ == "__main__":
    main()

