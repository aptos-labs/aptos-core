#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Per-block pipeline stage timings, one SVG per workload.

Grouped bars on a per-panel scale, drawn with the standard library alone. The
README's "Per-block charts" section covers why.
"""

import math
import os

# Fixed order, fixed color. Categorical slots 1-3, validated for CVD
# separation and for contrast against both surfaces.
SERIES = (
    ("execution", "execution_ms", "s0"),
    ("ledger update", "ledger_update_ms", "s1"),
    ("commit", "commit_ms", "s2"),
)

MARGIN_LEFT = 58
MARGIN_RIGHT = 20
PAD_TOP = 94
PANEL_TITLE_H = 24
PLOT_H = 300
X_AXIS_H = 26
PANEL_GAP = 24
PAD_BOTTOM = 32

GRIDLINES = 5
BAR_GAP = 2.0
BAR_RADIUS = 4.0
GROUP_FILL = 0.78
LABEL_FONT = 11.0
SUBTITLE_FONT = 11.5

STYLE = """
  .surface { fill: #fcfcfb; }
  .grid { stroke: #e5e5e3; stroke-width: 1; }
  .axis { stroke: #d6d5d1; stroke-width: 1; }
  .ink { fill: #0b0b0b; }
  .ink2 { fill: #52514e; }
  /* A direct label can land over a bar, so it carries a surface-colored
     outline to stay legible without wearing the series color. */
  .halo { stroke: #fcfcfb; stroke-width: 3.5; stroke-linejoin: round;
          paint-order: stroke; }
  .s0 { fill: #2a78d6; }
  .s1 { fill: #eb6834; }
  .s2 { fill: #1baf7a; }
  @media (prefers-color-scheme: dark) {
    .surface { fill: #1a1a19; }
    .grid { stroke: #33332f; }
    .axis { stroke: #4a4a46; }
    .ink { fill: #f5f5f3; }
    .ink2 { fill: #a8a7a3; }
    .halo { stroke: #1a1a19; }
    .s0 { fill: #3987e5; }
    .s1 { fill: #d95926; }
    .s2 { fill: #199e70; }
  }
"""


def _esc(text):
    return (
        str(text)
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def _nice_axis(peak):
    """The y-axis ceiling and its gridline step.

    The ceiling is the peak itself, so the tallest bar fills the panel. The
    gridlines land on round values under it and the top one is wherever that
    falls; the direct label already names the peak.
    """
    if peak <= 0:
        return 1.0, 0.25
    target = peak / GRIDLINES
    magnitude = 10 ** math.floor(math.log10(target))
    step = min(
        (factor * magnitude for factor in (1, 2, 2.5, 5, 10)),
        key=lambda candidate: abs(candidate - target),
    )
    return peak, step


def _fmt_ms(value, step):
    if step >= 10:
        return f"{value:.0f}"
    if step >= 1:
        return f"{value:.1f}"
    return f"{value:.2f}"


def _bar_path(x, y, width, height):
    """A bar anchored to the baseline with rounded top corners."""
    radius = min(BAR_RADIUS, width / 2.0, height)
    return (
        f"M{x:.1f},{y + height:.1f}"
        f"V{y + radius:.1f}"
        f"A{radius:.1f},{radius:.1f} 0 0 1 {x + radius:.1f},{y:.1f}"
        f"H{x + width - radius:.1f}"
        f"A{radius:.1f},{radius:.1f} 0 0 1 {x + width:.1f},{y + radius:.1f}"
        f"V{y + height:.1f}Z"
    )


def _place_labels(labels, left, right, top):
    """Nudge direct labels apart. Only three per panel, so a linear sweep over
    the already-placed ones is enough.
    """
    placed = []
    for label in sorted(labels, key=lambda it: it["x"]):
        half = LABEL_FONT * 0.3 * len(label["text"]) + 4.0
        label["x"] = min(max(label["x"], left + half), right - half)
        while any(
            abs(label["x"] - other["x"]) < half + other["half"]
            and abs(label["y"] - other["y"]) < 13.0
            for other in placed
        ):
            label["y"] -= 13.0
        label["y"] = max(label["y"], top + 9.0)
        label["half"] = half
        placed.append(label)
    return placed


def _panel(blocks, title, top, plot_w, tick_every):
    """One VM's panel: title, gridlines, bars, direct labels, x axis."""
    plot_top = top + PANEL_TITLE_H
    baseline = plot_top + PLOT_H
    left = MARGIN_LEFT
    right = MARGIN_LEFT + plot_w
    ceiling, step = _nice_axis(
        max(block[key] for block in blocks for _, key, _ in SERIES)
    )
    out = [
        f'<text class="ink" x="{left}" y="{top + 14:.0f}" '
        f'font-size="13.5" font-weight="600">{_esc(title)}</text>',
        f'<text class="ink2" x="{right}" y="{top + 14:.0f}" font-size="11" '
        f'text-anchor="end">peak {_fmt_ms(ceiling, step)} ms</text>',
    ]

    value = 0.0
    while value <= ceiling:
        y = baseline - PLOT_H * value / ceiling
        cls = "axis" if value == 0.0 else "grid"
        out.append(
            f'<line class="{cls}" x1="{left}" y1="{y:.1f}" '
            f'x2="{right}" y2="{y:.1f}"/>'
        )
        out.append(
            f'<text class="ink2" x="{left - 8}" y="{y + 3.5:.1f}" '
            f'font-size="10.5" text-anchor="end">{_fmt_ms(value, step)}</text>'
        )
        value += step

    group_w = plot_w / len(blocks)
    inner_w = group_w * GROUP_FILL
    bar_w = max(1.5, (inner_w - BAR_GAP * (len(SERIES) - 1)) / len(SERIES))
    span_w = bar_w * len(SERIES) + BAR_GAP * (len(SERIES) - 1)

    peaks = {name: (-1.0, 0.0) for name, _, _ in SERIES}
    for index, block in enumerate(blocks):
        group_x = left + index * group_w + (group_w - span_w) / 2.0
        for slot, (name, key, cls) in enumerate(SERIES):
            value = block[key]
            x = group_x + slot * (bar_w + BAR_GAP)
            height = PLOT_H * value / ceiling
            if value > peaks[name][0]:
                peaks[name] = (value, x + bar_w / 2.0)
            if height < 0.3:
                continue
            tip = f"block {index} · {name} · {value:.1f} ms"
            out.append(
                f'<path class="{cls}" '
                f'd="{_bar_path(x, baseline - height, bar_w, height)}">'
                f"<title>{_esc(tip)}</title></path>"
            )

        if index % tick_every == 0:
            out.append(
                f'<text class="ink2" x="{group_x + span_w / 2.0:.1f}" '
                f'y="{baseline + 15:.0f}" font-size="10.5" '
                f'text-anchor="middle">{index}</text>'
            )

    labels = [
        {
            "text": f"{name} {_fmt_ms(peaks[name][0], step)} ms",
            "x": peaks[name][1],
            "y": baseline - PLOT_H * peaks[name][0] / ceiling - 6.0,
        }
        for name, _, _ in SERIES
        if peaks[name][0] > 0
    ]
    for label in _place_labels(labels, left, right, plot_top):
        out.append(
            f'<text class="ink2 halo" x="{label["x"]:.1f}" '
            f'y="{label["y"]:.1f}" font-size="{LABEL_FONT:.0f}" '
            f'text-anchor="middle">{_esc(label["text"])}</text>'
        )
    return out


def render(workload, v1_blocks, mono_blocks):
    """The chart for one workload, as an SVG document."""
    count = min(len(v1_blocks), len(mono_blocks))
    v1_blocks, mono_blocks = v1_blocks[:count], mono_blocks[:count]

    subtitle = (
        f"{count} blocks, median run per VM. Each panel is scaled to its own "
        f"peak. Stages run concurrently and do not sum to block latency."
    )
    # A short workload would otherwise be narrower than its own subtitle.
    plot_w = max(720.0, count * 40.0, SUBTITLE_FONT * 0.52 * len(subtitle))
    width = MARGIN_LEFT + plot_w + MARGIN_RIGHT
    panel_h = PANEL_TITLE_H + PLOT_H + X_AXIS_H
    height = PAD_TOP + 2 * panel_h + PANEL_GAP + PAD_BOTTOM

    tick_every = max(1, math.ceil(count / 15))

    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" '
        f'height="{height:.0f}" viewBox="0 0 {width:.0f} {height:.0f}" '
        f'font-family="ui-sans-serif, -apple-system, Helvetica, Arial, '
        f'sans-serif">',
        f"<style>{STYLE}</style>",
        f'<rect class="surface" x="0" y="0" width="{width:.0f}" '
        f'height="{height:.0f}"/>',
        f'<text class="ink" x="{MARGIN_LEFT}" y="28" font-size="15" '
        f'font-weight="600">{_esc(workload)} — per-block stage time</text>',
        f'<text class="ink2" x="{MARGIN_LEFT}" y="48" '
        f'font-size="{SUBTITLE_FONT}">{_esc(subtitle)}</text>',
    ]

    legend_x = MARGIN_LEFT
    for name, _, cls in SERIES:
        out.append(
            f'<rect class="{cls}" x="{legend_x:.0f}" y="64" width="10" '
            f'height="10" rx="2"/>'
        )
        out.append(
            f'<text class="ink2" x="{legend_x + 15:.0f}" y="73" '
            f'font-size="{SUBTITLE_FONT}">{_esc(name)}</text>'
        )
        legend_x += 15 + SUBTITLE_FONT * 0.56 * len(name) + 22

    out += _panel(v1_blocks, "V1 MoveVM", PAD_TOP, plot_w, tick_every)
    out += _panel(
        mono_blocks,
        "MonoMove",
        PAD_TOP + panel_h + PANEL_GAP,
        plot_w,
        tick_every,
    )
    out.append(
        f'<text class="ink2" x="{MARGIN_LEFT + plot_w / 2:.0f}" '
        f'y="{height - PAD_BOTTOM + 20:.0f}" font-size="11" '
        f'text-anchor="middle">block</text>'
    )
    out.append("</svg>")
    return "\n".join(out) + "\n"


def write_charts(chart_dir, charts):
    """Render `(workload, v1_blocks, mono_blocks)` triples into `chart_dir`.

    Returns the filenames written, in the order given.
    """
    os.makedirs(chart_dir, exist_ok=True)
    written = []
    for workload, v1_blocks, mono_blocks in charts:
        if not v1_blocks or not mono_blocks:
            continue
        name = f"{workload}.svg"
        with open(os.path.join(chart_dir, name), "w", encoding="utf-8") as f:
            f.write(render(workload, v1_blocks, mono_blocks))
        written.append(name)
    return written
