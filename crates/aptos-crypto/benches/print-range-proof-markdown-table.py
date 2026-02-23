#!/usr/bin/env python3
import sys, csv, re, math
from collections import defaultdict

HEADER = [
    "Scheme",
    "n",
    "Proving time (ms)",
    "Verify time (ms)",
    "Total time (ms)",
    "Proof size (bytes)",
]

BP_NAME = "Bulletproofs"
DE_NAME = "univ DeKART (BLS12-381)"
DE_MV_NAME = "multiv DeKART (BLS12-381)"

BP_PROVE_ID = "range_prove"
BP_VERIFY_ID = "range_verify"
DE_PROVE_ID = "prove"
DE_VERIFY_ID = "verify"

bp_re = re.compile(r"batch=(\d+)_bits=(\d+)")
de_re = re.compile(r"ell=(\d+)_n=(\d+)")


def bp_proof_size(n, ell):
    """Bulletproofs proof size = 32 * (9 + 2 log2(n·ell))"""
    return int(32 * (9 + 2 * math.log2(n * ell)))

def de_proof_size(n, ell):
    """DeKART proof size = (ell+5)*48 + (ell+4)*32"""
    return int((ell + 5) * 48 + (ell + 4) * 32)

def read_rows(fp):
    reader = csv.DictReader(fp)
    for row in reader:
        for k in row:
            if isinstance(row[k], str):
                row[k] = row[k].strip()
        yield row

def parse_param_bulletproofs(param):
    m = bp_re.fullmatch(param)
    if not m: return None
    n = int(m.group(1))
    ell = int(m.group(2))
    return n, ell

def parse_param_dekart(param):
    m = de_re.fullmatch(param)
    if not m: return None
    ell = int(m.group(1))
    n = int(m.group(2))
    return n, ell

def ns_to_ms(ns):
    return round(float(ns) / 1e6, 2)

def fmt_ms(x):
    return f"{x:,.2f}"

def fmt_ratio(r):
    return f"{r:.2f}x"

def fmt_int(x: int) -> str:
    return f"{x:,}"

GREEN = 'color:#15803d'  # bold green when faster
RED   = 'color:#dc2626'  # red when slower

def decorate_dekart(value_ms, ratio):
    """
    Returns:
      display_text: plain text '1,234.56 (1.20x)' for width calculation
      render_text:  HTML-styled number only; ratio stays uncolored
                    faster (ratio < 1.0): bold green number
                    slower (ratio > 1.0): red number
                    equal  (ratio == 1.0): unstyled number
    """
    num_txt = fmt_ms(value_ms)
    ratio_txt = f" ({fmt_ratio(ratio)})"
    display = num_txt + ratio_txt

    if ratio < 1.0:
        # slower → red number, ratio uncolored
        render = f"<span style=\"{RED}\">{num_txt}</span>{ratio_txt}"
    elif ratio > 1.0:
        # faster → bold green number, ratio uncolored
        render = f"<span style=\"{GREEN}; font-weight:700\">{num_txt}</span>{ratio_txt}"
    else:
        # equal → no styling
        render = display

    return display, render

def decorate_dekart_size(value_bytes: int, ratio: float):
    """
    Like decorate_dekart, but for integer byte sizes.
    Colors ONLY the number; leaves ' (1.23x)' uncolored.
    Uses your current color rule inside decorate_dekart (green if ratio>1, red if ratio<1).
    """
    num_txt = fmt_int(value_bytes)
    ratio_txt = f" ({fmt_ratio(ratio)})"
    display = num_txt + ratio_txt

    # Reuse your current logic: red when ratio > 1.0, green when ratio < 1.0
    if ratio < 1.0:
        render = f"<span style=\"{GREEN}; font-weight:700\">{num_txt}</span>{ratio_txt}"
    elif ratio > 1.0:
        render = f"<span style=\"{RED}\">{num_txt}</span>{ratio_txt}"
    else:
        render = display

    return display, render

def next_pow2_ge(x: int) -> int:
    """Small helper: next power of two >= x."""
    if x <= 1:
        return 1
    p = 1
    while p < x:
        p <<= 1
    return p

def accumulate(rows):
    """
    Build dicts keyed by (n, ell) -> {"prove": ns, "verify": ns}
    separately for Bulletproofs and DeKART.
    """
    bp = defaultdict(dict)
    de = defaultdict(dict)
    de_mv = defaultdict(dict)
    ells_seen = set()

    for r in rows:
        group = r.get("Group", "")
        ident  = r.get("Id", "")
        param  = r.get("Parameter", "")
        mean_ns = r.get("Mean(ns)", "")

        if mean_ns == "" or mean_ns is None:
            continue
        try:
            mean_ns = float(mean_ns)
        except ValueError:
            continue

        if group == "bulletproofs":
            parsed = parse_param_bulletproofs(param)
            if not parsed: continue
            n, ell = parsed
            ells_seen.add(ell)
            if ident == BP_PROVE_ID:
                bp[(n, ell)]["prove"] = mean_ns
            elif ident == BP_VERIFY_ID:
                bp[(n, ell)]["verify"] = mean_ns

        elif "dekart-multivar" in group or "dekart_multivar" in group:
            parsed = parse_param_dekart(param)
            if not parsed: continue
            n, ell = parsed
            ells_seen.add(ell)
            if ident == DE_PROVE_ID:
                de_mv[(n, ell)]["prove"] = mean_ns
            elif ident == DE_VERIFY_ID:
                de_mv[(n, ell)]["verify"] = mean_ns
        elif group.startswith("dekart"):
            parsed = parse_param_dekart(param)
            if not parsed: continue
            n, ell = parsed
            ells_seen.add(ell)
            if ident == DE_PROVE_ID:
                de[(n, ell)]["prove"] = mean_ns
            elif ident == DE_VERIFY_ID:
                de[(n, ell)]["verify"] = mean_ns

    return bp, de, de_mv, sorted(ells_seen)

def make_rows_for_ell(bp_map, de_map, de_mv_map, ell):
    """
    For each n (across all schemes), emit BP row, univ DeKART row, multiv DeKART row when present.
    DeKART ratios are computed vs Bulletproofs at the *next power of two* batch size.
    """
    bp_ns = sorted(n for (n,e),dv in bp_map.items() if e==ell and "prove" in dv and "verify" in dv)
    de_ns = sorted(n for (n,e),dv in de_map.items() if e==ell and "prove" in dv and "verify" in dv)
    de_mv_ns = sorted(n for (n,e),dv in de_mv_map.items() if e==ell and "prove" in dv and "verify" in dv)

    bp_vals = {}
    for n in bp_ns:
        dv = bp_map[(n, ell)]
        p = ns_to_ms(dv["prove"]); v = ns_to_ms(dv["verify"]); t = round(p+v, 2)
        bp_vals[n] = (p, v, t)
    de_vals = {}
    for n in de_ns:
        dv = de_map[(n, ell)]
        p = ns_to_ms(dv["prove"]); v = ns_to_ms(dv["verify"]); t = round(p+v, 2)
        de_vals[n] = (p, v, t)
    de_mv_vals = {}
    for n in de_mv_ns:
        dv = de_mv_map[(n, ell)]
        p = ns_to_ms(dv["prove"]); v = ns_to_ms(dv["verify"]); t = round(p+v, 2)
        de_mv_vals[n] = (p, v, t)

    all_n = sorted(set(bp_ns) | set(de_ns) | set(de_mv_ns))
    out = []
    for n in all_n:
        if n in bp_ns:
            bp_p, bp_v, bp_t = bp_vals[n]
            bp_size = bp_proof_size(n, ell)
            out.append({
                "Scheme": BP_NAME,
                "n": str(n),
                "p_display": fmt_ms(bp_p), "p_render": fmt_ms(bp_p),
                "v_display": fmt_ms(bp_v), "v_render": fmt_ms(bp_v),
                "t_display": fmt_ms(bp_t), "t_render": fmt_ms(bp_t),
                "s_display": fmt_int(bp_size),
                "s_render":  fmt_int(bp_size),
            })
        if n in de_ns:
            de_p, de_v, de_t = de_vals[n]
            baseline_n = next_pow2_ge(n + 1)
            de_size = de_proof_size(n, ell)
            if baseline_n in bp_vals:
                bp_p, bp_v, bp_t = bp_vals[baseline_n]
                rp = bp_p / de_p if bp_p else float("inf")
                rv = bp_v / de_v if bp_v else float("inf")
                rt = bp_t / de_t if bp_t else float("inf")
                p_disp, p_rend = decorate_dekart(de_p, rp)
                v_disp, v_rend = decorate_dekart(de_v, rv)
                t_disp, t_rend = decorate_dekart(de_t, rt)
                bp_size = bp_proof_size(baseline_n, ell)
                rs = de_size / bp_size if bp_size else float("inf")
                s_disp, s_rend = decorate_dekart_size(de_size, rs)
            else:
                p_disp = p_rend = fmt_ms(de_p)
                v_disp = v_rend = fmt_ms(de_v)
                t_disp = t_rend = fmt_ms(de_t)
                s_disp = s_rend = fmt_int(de_size)
            out.append({
                "Scheme": DE_NAME,
                "n": str(n),
                "p_display": p_disp, "p_render": p_rend,
                "v_display": v_disp, "v_render": v_rend,
                "t_display": t_disp, "t_render": t_rend,
                "s_display": s_disp, "s_render": s_rend,
            })
        if n in de_mv_ns:
            de_p, de_v, de_t = de_mv_vals[n]
            baseline_n = next_pow2_ge(n + 1)
            de_size = de_proof_size(n, ell)
            if baseline_n in bp_vals:
                bp_p, bp_v, bp_t = bp_vals[baseline_n]
                rp = bp_p / de_p if bp_p else float("inf")
                rv = bp_v / de_v if bp_v else float("inf")
                rt = bp_t / de_t if bp_t else float("inf")
                p_disp, p_rend = decorate_dekart(de_p, rp)
                v_disp, v_rend = decorate_dekart(de_v, rv)
                t_disp, t_rend = decorate_dekart(de_t, rt)
                bp_size = bp_proof_size(baseline_n, ell)
                rs = de_size / bp_size if bp_size else float("inf")
                s_disp, s_rend = decorate_dekart_size(de_size, rs)
            else:
                p_disp = p_rend = fmt_ms(de_p)
                v_disp = v_rend = fmt_ms(de_v)
                t_disp = t_rend = fmt_ms(de_t)
                s_disp = s_rend = fmt_int(de_size)
            out.append({
                "Scheme": DE_MV_NAME,
                "n": str(n),
                "p_display": p_disp, "p_render": p_rend,
                "v_display": v_disp, "v_render": v_rend,
                "t_display": t_disp, "t_render": t_rend,
                "s_display": s_disp, "s_render": s_rend,
            })
    return out

def padded_table(rows):
    """
    Compute widths from the plain display strings, then emit
    padded Markdown rows with the render strings (HTML-styled).
    """
    cols = HEADER
    display_map = {
        "Scheme": "Scheme",
        "n": "n",
        "Proving time (ms)": "p_display",
        "Verify time (ms)":  "v_display",
        "Total time (ms)":   "t_display",
        "Proof size (bytes)": "s_display",
    }
    render_map = {
        "Scheme": "Scheme",
        "n": "n",
        "Proving time (ms)": "p_render",
        "Verify time (ms)":  "v_render",
        "Total time (ms)":   "t_render",
        "Proof size (bytes)": "s_render",
    }

    widths = {c: len(c) for c in cols}
    for r in rows:
        for c in cols:
            widths[c] = max(widths[c], len(str(r.get(display_map[c], ""))))

    left_cols = {"Scheme", "n", "Proof size (bytes)"}
    right_cols = {"Proving time (ms)", "Verify time (ms)", "Total time (ms)"}

    def pad(c, s, align):
        s = str(s)
        if align == "right":
            return " " + s.rjust(widths[c]) + " "
        return " " + s.ljust(widths[c]) + " "

    header_line = "|" + "|".join(pad(c, c, "left") for c in cols) + "|"
    sep_line    = "|" + "|".join("-" * (widths[c] + 2) for c in cols) + "|"

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

    bp_map, de_map, de_mv_map, ells = accumulate(rows)

    first = True
    for ell in ells:
        tbl_rows = make_rows_for_ell(bp_map, de_map, de_mv_map, ell)
        if not tbl_rows:
            continue
        if not first:
            print()
        first = False
        print(f"#### $\\ell = {ell}$ numbers\n")
        print(padded_table(tbl_rows))

if __name__ == "__main__":
    main()

