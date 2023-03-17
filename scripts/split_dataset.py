#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

def main(sorted_datapoints, cut_point):
    n = len(sorted_datapoints)
    i = 0
    while i < n and sorted_datapoints[i][0] < cut_point: i+=1
    return sorted_datapoints[:i], sorted_datapoints[i:]

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--cut_point', required=True, type=float)
    parser.add_argument('--save_to_file', action='store_true')
    args = parser.parse_args()
    dataset = json.loads(Path(args.dataset_path).read_text())
    dataset_lo, dataset_hi = main(dataset, args.cut_point)
    print(json.dumps({'lo':dataset_lo, 'hi':dataset_hi}))
    if args.save_to_file:
        path_lo = Path(f'{args.dataset_path}.until.{args.cut_point}.json')
        path_hi = Path(f'{args.dataset_path}.since.{args.cut_point}.json')
        path_lo.write_text(json.dumps(dataset_lo))
        path_hi.write_text(json.dumps(dataset_hi))
        print(f'Saved to {path_lo}')
        print(f'Saved to {path_hi}')
