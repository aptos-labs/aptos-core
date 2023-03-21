#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

def main(datasets):
    best = {}
    for dataset in datasets:
        for x,y in dataset:
            best[x] = min(y, best[x]) if x in best else y
    new_dataset = [(k,v) for k,v in best.items()]
    new_dataset.sort()
    return new_dataset

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('dataset_paths', nargs='+')
    args = parser.parse_args()
    datasets = [json.loads(Path(path).read_text()) for path in args.dataset_paths]
    new_dataset = main(datasets)
    jsonstr = json.dumps(new_dataset)
    print(jsonstr)
    print()
    # Save to file.
    x_min = new_dataset[0][0]
    x_max = new_dataset[-1][0]
    out_path = Path(f'union.{x_min}-{x_max+1}.json')
    out_path.write_text(jsonstr)
    print(f'Saved to {out_path}.')
    print()
