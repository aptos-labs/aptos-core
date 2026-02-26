#!/usr/bin/env python3
import sys, csv, re, os, json
from collections import defaultdict

HEADER = [
    "Scheme",
    "Ell",  # <-- need capital E here to be consistent with code later
    "Setup",
    "Transcript size",
    "Deal (ms)",
    "Serialize (ms)",
    "Aggregate (ms)",
    "Verify (ms)",
    "Decrypt-share (ms)",
]

# Maps internal column names to their rendered display names (e.g. LaTeX-style Markdown)
HEADER_DISPLAY = {
    "Ell": r"$\ell$",
}

V1_NAME = "Chunky"
V2_NAME = "Chunky2"

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


def fmt_setup(setup):
    """
    Transform a setup string like 'weighted_129-out-of-219_136-players'
    into '129-out-of-219 / 136 players'. Leaves unrecognized formats unchanged.
    """
    m = re.match(r'^(?:[^_]+_)?(\d+-out-of-\d+)_(\d+)-players$', setup)
    if m:
        return f"{m.group(1)} / {m.group(2)} players"
    return setup


def humanize_bytes(n):
    if n is None or n == "—":
        return "—"
    n = int(n)
    if n >= 1024 * 1024:
        return f"{n / (1024 * 1024):,.2f} MiB"
    elif n >= 1024:
        return f"{n / 1024:.2f} KiB"
    else:
        return f"{n} B"


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

def parse_ell(group):
    """
    Extract ell from Group.
    Examples:
      pvss_chunky_v1_bls12-381_16  -> 16
      pvss/chunky_v1/bls12-381/16  -> 16
      pvss_chunky_v1_bls12-381     -> None
    """
    m = re.search(r"(?:/|_)(\d+)$", group)
    return m.group(1) if m else None

def parse_transcript_bytes_from_folder(folder_path):
    """
    Extract transcript_bytes from benchmark.json file in the folder.
    The folder_path should be relative to the current directory (target/criterion).
    """
    benchmark_json = os.path.join(folder_path, "base", "benchmark.json")
    if not os.path.exists(benchmark_json):
        # Try "new" directory if "base" doesn't exist
        benchmark_json = os.path.join(folder_path, "new", "benchmark.json")
    
    if os.path.exists(benchmark_json):
        try:
            with open(benchmark_json, 'r') as f:
                data = json.load(f)
                # Extract from function_id field
                if 'function_id' in data:
                    function_id = data['function_id']
                    m = re.search(r'transcript_bytes=(\d+)$', function_id)
                    if m:
                        return int(m.group(1))
        except (json.JSONDecodeError, IOError, ValueError):
            # If the benchmark.json file is missing, unreadable, or malformed,
            # treat it as "no transcript_bytes" and fall back to returning None.
            pass
    
    return None


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
    if parameter and parameter.strip():
        return parameter.strip()
    
    for op in OPERATIONS:
        if ident.startswith(op + "/"):
            setup = ident[len(op) + 1:]
        elif ident.startswith(op + "_"):
            setup = ident[len(op) + 1:]
        else:
            continue
        
        # Strip /transcript_bytes=NNN if present
        setup = re.sub(r"(/|_)transcript_bytes=\d+$", "", setup)
        return setup
    
    return ident

def build_folder_map():
    """
    Build a mapping: (group_name, operation, setup) -> folder name with transcript_bytes.
    Searches inside pvss_* directories for folders matching operations.
    The setup is extracted from the folder name by removing the operation prefix and transcript_bytes suffix.
    """
    folder_map = {}
    for entry in os.listdir("."):
        if os.path.isdir(entry) and (entry.startswith("pvss_chunky_v1") or entry.startswith("pvss_chunky_v2")):
            group_name = entry
            group_path = os.path.join(".", entry)
            for subentry in os.listdir(group_path):
                subentry_path = os.path.join(group_path, subentry)
                if os.path.isdir(subentry_path):
                    for op in OPERATIONS:
                        if subentry.startswith(op) and "_transcript_bytes=" in subentry:
                            # Extract setup from folder name: "serialize_SETUP_transcript_bytes=NNN" -> "SETUP"
                            # Remove operation prefix
                            setup_part = subentry[len(op) + 1:]  # +1 for the underscore
                            # Remove transcript_bytes suffix
                            setup = re.sub(r"_transcript_bytes=\d+$", "", setup_part)
                            key = (group_name, op, setup)
                            # Store the full path relative to current directory
                            folder_path = os.path.join(group_name, subentry)
                            folder_map[key] = folder_path
                            break
    return folder_map


def accumulate(rows, folder_map=None):
    """
    Build nested dict: (setup, ell) -> version -> operation -> time_ns.
    Also stores transcript_bytes per version.
    """
    data = defaultdict(dict)

    for r in rows:
        group = r.get("Group", "")
        ident = r.get("Id", "")
        param = r.get("Parameter", "")
        mean_ns = r.get("Mean(ns)", "")

        if mean_ns in ("", None):
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

        ell = parse_ell(group)
        setup = parse_setup(ident, param)

        # Only serialize operations have transcript_bytes in folder names
        tx_bytes = None
        if folder_map and operation == "serialize":
            key = (group, operation, setup)
            if key in folder_map:
                folder_path = folder_map[key]
                tx_bytes = parse_transcript_bytes_from_folder(folder_path)

        # Initialize dict for this setup/ell if missing
        if (setup, ell) not in data:
            data[(setup, ell)] = {"v1": {}, "v2": {}}

        # Store mean_ns
        data[(setup, ell)][version][operation] = mean_ns

        # Only set tx_bytes if we actually found a value
        if tx_bytes is not None:
            data[(setup, ell)][version]["tx_bytes"] = tx_bytes

    return data


def make_rows_for_setup(setup, ell, v1_data, v2_data):
    """
    Create rows comparing v1 and v2 for a single setup.
    """
    rows = []

    # Check if we have all operations for both versions
    v1_complete = all(op in v1_data for op in OPERATIONS)
    v2_complete = all(op in v2_data for op in OPERATIONS)

    if not v1_complete and not v2_complete:
        return rows

    v1_tx_bytes = humanize_bytes(v1_data.get("tx_bytes", "—"))
    v2_tx_bytes = humanize_bytes(v2_data.get("tx_bytes", "—"))

    # Build row for v1
    if v1_complete:
        v1_row = {
            "Scheme": V1_NAME,
            "Ell": ell or "—",
            "Setup": fmt_setup(setup),
            "Transcript size": v1_tx_bytes,
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
            "Ell": ell or "—",
            "Setup": fmt_setup(setup),
            "Transcript size": v2_tx_bytes,
        }
        for op in OPERATIONS:
            if op not in v2_data:
                continue  # Skip missing operations
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
        "Ell": "Ell",
        "Setup": "Setup",
        "Transcript size": "Transcript size",
        "Deal (ms)": "deal_display",
        "Serialize (ms)": "serialize_display",
        "Aggregate (ms)": "aggregate_display",
        "Verify (ms)": "verify_display",
        "Decrypt-share (ms)": "decrypt-share_display",
    }
    render_map = {
        "Scheme": "Scheme",
        "Ell": "Ell",
        "Setup": "Setup",
        "Transcript size": "Transcript size",
        "Deal (ms)": "deal_render",
        "Serialize (ms)": "serialize_render",
        "Aggregate (ms)": "aggregate_render",
        "Verify (ms)": "verify_render",
        "Decrypt-share (ms)": "decrypt-share_render",
    }

    widths = {c: len(HEADER_DISPLAY.get(c, c)) for c in cols}
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

    header_line = "|" + "|".join(pad(c, HEADER_DISPLAY.get(c, c), "left") for c in cols) + "|"
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
    """
    Main function: reads CSV, builds folder map, accumulates data, and prints tables.
    Expects to be run from target/criterion directory.
    """
    # Validate we can find pvss benchmark directories
    pvss_dirs = [d for d in os.listdir(".") if os.path.isdir(d) and (d.startswith("pvss_chunky_v1") or d.startswith("pvss_chunky_v2"))]
    if not pvss_dirs:
        print("Error: No pvss_chunky_v1 or pvss_chunky_v2 directories found in current directory.", file=sys.stderr)
        print("Please run this script from target/criterion directory.", file=sys.stderr)
        sys.exit(1)

    # Read CSV data
    if len(sys.argv) > 1 and sys.argv[1] != "-":
        with open(sys.argv[1], newline="") as f:
            rows = list(read_rows(f))
    else:
        rows = list(read_rows(sys.stdin))

    # Build folder map and accumulate benchmark data
    folder_map = build_folder_map()
    data = accumulate(rows, folder_map)

    if not data:
        print("No PVSS benchmark data found in CSV!", file=sys.stderr)
        sys.exit(1)

    # Generate tables for each setup/ell combination
    keys = sorted(data.keys(), key=lambda x: (x[0], x[1] or ""))
    tables = []

    for setup, ell in keys:
        v1_data = data[(setup, ell)].get("v1", {})
        v2_data = data[(setup, ell)].get("v2", {})
        tbl_rows = make_rows_for_setup(setup, ell, v1_data, v2_data)
        if tbl_rows:
            tables.append(padded_table(tbl_rows))

    if not tables:
        print("No complete benchmark data found!", file=sys.stderr)
        sys.exit(1)

    # Print all tables
    print("\n\n".join(tables))
if __name__ == "__main__":
    main()