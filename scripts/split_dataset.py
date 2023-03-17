#!/usr/bin/env python3

import argparse
import json
from pathlib import Path
import re

def parse_range_from_dataset_path(dataset_path):
    match = re.match(r'(.+)\.(\d+)-(\d+)\.json', dataset_path)
    assert match!=None
    base_path = match.group(1)
    x_begin = int(match.group(2))
    x_end =  int(match.group(3))
    return (base_path, x_begin, x_end)

def main(sorted_datapoints, cut_point):
    n = len(sorted_datapoints)
    i = 0
    while i < n and sorted_datapoints[i][0] < cut_point: i+=1
    return sorted_datapoints[:i], sorted_datapoints[i:]

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--cut_point', required=True, type=int)
    args = parser.parse_args()
    base_path, x_begin, x_end = parse_range_from_dataset_path(args.dataset_path)
    assert x_begin <= args.cut_point < x_end
    dataset = json.loads(Path(args.dataset_path).read_text())
    sub_dataset_left, sub_dataset_right = main(dataset, args.cut_point)
    print(json.dumps({'left':sub_dataset_left, 'right':sub_dataset_right}))

    # Save to files.
    path_to_left = Path(f'{base_path}.{x_begin}-{args.cut_point}.json')
    path_to_right = Path(f'{base_path}.{args.cut_point}-{x_end}.json')
    path_to_left.write_text(json.dumps(sub_dataset_left))
    path_to_right.write_text(json.dumps(sub_dataset_right))
    print(f'Saved to {path_to_left}')
    print(f'Saved to {path_to_right}')
