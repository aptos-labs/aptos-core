#!/usr/bin/env python3
"""Summarize ab.tsv. Reports max over reps, plus each arm's rep spread."""
import sys, csv, collections

rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t'))
want = [a.split('=')[0] for a in sys.argv[2:]] or None
by = collections.defaultdict(list)
order = []
for r in rows:
    if want and r['label'] not in want:
        continue
    by[(r['label'], r['workload'], r['mode'])].append(r)
    if r['label'] not in order:
        order.append(r['label'])


def vals(rs, k):
    return sorted(float(x[k]) for x in rs if x[k] not in ('NA', ''))


print(f"{'label':<24}{'workload':<12}{'legacy':>9}{'mono':>9}{'speedup':>9}   mono reps")
for lab in order:
    for wl in ('bench-clob', 'bench-aave'):
        lg, mo = vals(by.get((lab, wl, 'legacy'), []), 'inner_tps'), vals(by.get((lab, wl, 'mono'), []), 'inner_tps')
        if not lg or not mo:
            continue
        spread = ' '.join(f'{v:.0f}' for v in mo)
        print(f"{lab:<24}{wl:<12}{lg[-1]:>9.0f}{mo[-1]:>9.0f}{mo[-1] / lg[-1]:>8.2f}x   {spread}")
