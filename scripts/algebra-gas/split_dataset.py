#!/usr/bin/env python3

import argparse
import json
from pathlib import Path
import re
from time import time
import utils

def main(sorted_datapoints, cut_point):
    '''Split a dataset (sorted list of (x,y)) into 2, cutting at `x=cut_point`.'''
    n = len(sorted_datapoints)
    i = 0
    while i < n and sorted_datapoints[i][0] < cut_point: i+=1
    return sorted_datapoints[:i], sorted_datapoints[i:]

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--cut_point', required=True, type=int)
    args = parser.parse_args()
    base_path, x_begin, x_end = utils.parse_range_from_dataset_path(args.dataset_path)
    assert x_begin <= args.cut_point < x_end
    dataset = json.loads(Path(args.dataset_path).read_text())
    sub_dataset_left, sub_dataset_right = main(dataset, args.cut_point)
    print(json.dumps({'left':sub_dataset_left, 'right':sub_dataset_right}))

    # Save to files.
    path_to_left = Path(f'{base_path}.{x_begin}-{args.cut_point}.json')
    path_to_right = Path(f'{base_path}.{args.cut_point}-{x_end}.json')
    print(f'Saving datasets to:')
    print()
    print(f'  {path_to_left}')
    print(f'  {path_to_right}')
    print()
    path_to_left.write_text(json.dumps(sub_dataset_left))
    path_to_right.write_text(json.dumps(sub_dataset_right))
