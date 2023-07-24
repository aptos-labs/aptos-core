#!/usr/bin/env python3

import argparse
import load_bench_ns
from glob import glob
import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import re
from time import time

def get_datapoint(bench_path):
    items = bench_path.split('/')
    if not items[-1].isdecimal(): return None
    arg = int(items[-1])
    ns = load_bench_ns.main(bench_path)
    return (arg,ns)

def main(bench_path):
    '''Parse benchmark results as datapoints.

    Param `bench_path` has to be a serial bench, (e.g. 'target/criterion/hash/SHA2-256').
    '''
    for sbp in glob(f'{bench_path}/*'):
        print(sbp)
    datapoints = [get_datapoint(sub_bench_path) for sub_bench_path in glob(f'{bench_path}/*')]
    datapoints = [dp for dp in datapoints if dp!=None]
    assert len(datapoints)>=1
    datapoints.sort()
    return datapoints

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_path', required=True)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    datapoints = main(args.bench_path)
    jsonstr = json.dumps(datapoints)
    print(jsonstr)
    print()
    # Save to file.
    bench_name = str(Path(args.bench_path).relative_to(Path('target/criterion')))
    formatted_bench_name = re.sub('[^0-9a-zA-Z]', '_', bench_name)
    x_min = datapoints[0][0]
    x_max = datapoints[-1][0]
    cur_time = int(time())
    out_path = Path(f'{formatted_bench_name}.{cur_time}.{x_min}-{x_max+1}.json')
    print(f'Saving dataset to:')
    print()
    print(f'  {out_path}')
    print()
    out_path.write_text(jsonstr)
    if args.plot:
        x_values, y_values = zip(*datapoints)
        X = np.array(x_values)
        Y = np.array(y_values)
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.legend()
        plt.show(block=True)
