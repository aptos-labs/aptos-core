#!/usr/bin/env python3

import argparse
import matplotlib.pyplot as plt
import numpy as np

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('dataset_paths', nargs='+')
    args = parser.parse_args()
    for dataset_path in args.dataset_paths:
        x_values, y_values = zip(*dataset)
        X = np.array(x_values)
        Y = np.array(y_values)
        plt.plot(X, Y, 'o', label=dataset_path, markersize=2)
    plt.legend()
    plt.show(block=True)
