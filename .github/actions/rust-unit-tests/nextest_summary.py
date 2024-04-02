#!/usr/bin/env python

import argparse
import json
import sys

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('nextest_json')
    ap.add_argument('summary_out_path')
    ap.add_argument('-f', '--fail', default=False, action='store_true')
    args = ap.parse_args()

    badlines = 0
    eventCounts = {}
    failnames = []
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
    with open(args.summary_out_path, 'at') as fout:
        for event in ('ok', 'ignored', 'failed'):
            ec = eventCounts.pop(event, 0)
            fout.write(f'| {event} | {ec} |\n')
        for event, ec in eventCounts.items():
            fout.write(f'| {event} | {ec} |\n')
        if badlines != 0:
            fout.write(f'| bad lines | {badlines} |\n')
        fout.write('\n')
        if failnames:
            failnames.sort()
            failshow = failnames
            if len(failnames) > 30:
                failshow = failnames[:30]
            fout.write('## Failed\n\n')
            for fn in failshow:
                fout.write(f'    {fn}\n')
            if len(failnames) > 30:
                fout.write(f'    ... and {len(failnames)-30} more\n')
            fout.write('\n')
    if eventCounts.get('failed',0) != 0:
        sys.exit(1)
    if args.fail:
        sys.exit(1)

if __name__ == '__main__':
    main()
