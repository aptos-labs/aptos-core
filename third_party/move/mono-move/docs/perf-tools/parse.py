#!/usr/bin/env python3
"""Extract one TSV row from an executor-benchmark log."""
import re, sys

log, label, wl, mode, rep = sys.argv[1:6]
t = open(log, errors='replace').read()


def grab(p, d='NA'):
    m = re.search(p, t)
    return m.group(1) if m else d


inner = grab(r'\[main\].*fraction of execution [\d.]+ in inner block executor \(component TPS: ([\d.]+)\)')
execf = grab(r'\[main\].*fraction of total: [\d.]+ in execution \(component TPS: ([\d.]+)\)')
overall = grab(r'Overall TPS: ([\d.]+)')
txns = grab(r'over (\d+) txns')
print('\t'.join([label, wl, mode, rep, overall, inner, execf, txns]))
