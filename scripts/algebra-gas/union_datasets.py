#!/usr/bin/env python3

import argparse
import json
from pathlib import Path
import utils

def main(datasets):
    '''Union a list of datasets (each is a list of (x,y)).
    
    If multiple datapoints are present on the same x, take the minimum.
    '''
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
    base_path,_,_ = utils.parse_range_from_dataset_path(args.dataset_paths[0])
    new_dataset = main(datasets)
    jsonstr = json.dumps(new_dataset)
    print(jsonstr)
    print()
    # Save to file.
    x_min = new_dataset[0][0]
    x_max = new_dataset[-1][0]
    out_path = Path(f'{base_path}.{x_min}-{x_max+1}.json')
    print(f'Saving dataset to:')
    print()
    print(f'  {out_path}')
    print()
    out_path.write_text(jsonstr)
