#!/usr/bin/env python3

import argparse
import load_bench_ns
from glob import glob
import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

def get_datapoint(bench_path):
    items = bench_path.split('/')
    if not items[-1].isdecimal(): return None
    arg = int(items[-1])
    ns = load_bench_ns.main(bench_path)
    return (arg,ns)

def main(bench_path):
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
    print(json.dumps(datapoints))
    if args.plot:
        x_values, y_values = zip(*datapoints)
        X = np.array(x_values)
        Y = np.array(y_values)
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.legend()
        plt.show(block=True)
