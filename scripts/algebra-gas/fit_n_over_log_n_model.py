#!/usr/bin/env python3

import argparse
import json
from math import log2
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

def t(x):
    return x/log2(x)
def main(datapoints:list):
    for item in datapoints:
        assert type(item)==list
        assert len(item)==2
    x_values, y_values = zip(*datapoints)
    k = len([x for x in x_values if x <= 2])
    x_values = x_values[k:]
    y_values = y_values[k:]
    t_values = [t(x) for x in x_values]
    X = np.array(x_values)
    T = np.array(t_values)
    Y = np.array(y_values)
    A = np.vstack([T, np.ones(len(X))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, Y, rcond=None)
    k,b = sol
    return (X, T, Y, k, b)

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    jsonstr = Path(args.dataset_path).read_text()
    datapoints = json.loads(jsonstr)
    X,T,Y,k,b = main(datapoints)
    jsonstr = json.dumps({'model_type':'n_over_log_n','b':b, 'k':k})
    print(jsonstr)
    print()
    dataset_base_path = args.dataset_path.replace('.json', '')
    out_path = Path(f'{dataset_base_path}.model.json')
    out_path.write_text(jsonstr)
    print(f'Saved to {out_path}. Rename it if needed.')
    print()
    if args.plot:
        plt.plot(X, Y, 'o', label='dataset', markersize=2)
        plt.plot(X, k*T+b, 'r', label='fitted')
        plt.legend()
        plt.show(block=True)
