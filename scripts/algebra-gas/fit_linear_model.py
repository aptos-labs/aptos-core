#!/usr/bin/env python3

import argparse
import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
from time import time

def main(datapoints:list):
    '''Least-squares fit a line to a given dataset.'''
    for item in datapoints: assert len(item)==2
    x_values, y_values = zip(*datapoints)
    X = np.array(x_values)
    Y = np.array(y_values)
    A = np.vstack([X, np.ones(len(X))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, Y, rcond=None)
    k,b = sol
    return (X, Y, k, b)

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    jsonstr = Path(args.dataset_path).read_text()
    datapoints = json.loads(jsonstr)
    X,Y,k,b = main(datapoints)
    jsonstr = json.dumps({'b':b, 'k':k})
    print(jsonstr)
    print()
    dataset_base_path = args.dataset_path.replace('.json', '')
    cur_time = int(time())
    out_path = Path(f'{dataset_base_path}.model.{cur_time}.json')
    print(f'Saving model to:')
    print()
    print(f'  {out_path}')
    print()
    out_path.write_text(jsonstr)
    if args.plot:
        plt.plot(X, Y, 'o', label='dataset', markersize=2)
        plt.plot(X, k*X+b, 'r', label='fitted')
        plt.legend()
        plt.show(block=True)
