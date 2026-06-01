#!/usr/bin/env python

import argparse
import json
import sys

# how many failed tests to name before it's too many ...
FAIL_HEAD_LINES = 20

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('nextest_json')
    ap.add_argument('summary_out_path')
    ap.add_argument('-f', '--fail', default=False, action='store_true')
    args = ap.parse_args()

    badlines = 0
    eventCounts = {}
    failnames = []
    flakey = []
    with open(args.nextest_json, 'rt') as fin:
        for line in fin:
            try:
                rec = json.loads(line)
            except Exception as e:
                badlines += 1
                if badlines < 10:
                    print(e)
                continue
            rectype = rec.get('type')
            if rectype != 'test':
                continue
            event = rec.get('event')
            if event == 'started':
                continue
            eventCounts[event] = eventCounts.get(event, 0) + 1
            if event == 'failed':
                failnames.append(rec.get('name', '_'))
            if event == 'ok':
                testname = rec.get('name', '_')
                if '#' in testname:
                    # flakey test passed on retry
                    flakey.append(rec)
    with open(args.summary_out_path, 'at') as fout:
        rows = []
        for event in ('ok', 'ignored', 'failed'):
            ec = eventCounts.pop(event, 0)
            style = ""
            if event == 'failed':
                style = ' style="font-weight:bold;font-size:120%;color:#f00;"'
            rows.append(f'<tr{style}><td>{ec}</td><td>{event}</td></tr>')
        for event, ec in eventCounts.items():
            rows.append(f'<tr><td>{ec}</td><td>{event}</td></tr>')
        if badlines != 0:
            rows.append(f'<tr><td>{badlines}</td><td>bad lines</td></tr>')
        fout.write('<table>' + ''.join(rows) + '</table>\n\n')
        if failnames:
            failnames.sort()
            failshow = failnames
            if len(failnames) > FAIL_HEAD_LINES:
                failshow = failnames[:FAIL_HEAD_LINES]
            fout.write('## Failed\n\n')
            for fn in failshow:
                fout.write(f'    {fn}\n')
            if len(failnames) > FAIL_HEAD_LINES:
                fout.write(f'    ... and {len(failnames)-FAIL_HEAD_LINES} more\n')
            fout.write('\n')
        elif flakey:
            flakeshow = flakey
            if len(flakeshow) > FAIL_HEAD_LINES:
                flakeshow = flakeshow[:FAIL_HEAD_LINES]
            fout.write("## Flakey\n\n")
            for rec in flakeshow:
                name = rec['name']
                etime = rec.get('exec_time', '')
                fout.write(f"    {name} ({etime})\n")
            if len(flakey) > FAIL_HEAD_LINES:
                fout.write(f"    ... and {len(flakey)-FAIL_HEAD_LINES} more\n")
            fout.write("\n")
    if failnames:
        print(f"{len(failnames)} FAILING tests:")
        print("\n".join(failnames))
        print(f"{len(failnames)} FAILING tests")
    if eventCounts.get('failed',0) != 0:
        sys.exit(1)
    if args.fail:
        sys.exit(1)

if __name__ == '__main__':
    main()
