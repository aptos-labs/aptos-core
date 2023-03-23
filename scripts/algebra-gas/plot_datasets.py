#!/usr/bin/env python3

'''
This module plots an existing dataset.
'''

import argparse
import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('dataset_paths', nargs='+')
    args = parser.parse_args()
    for dataset_path in args.dataset_paths:
        dataset = json.loads(Path(dataset_path).read_text())
        x_values, y_values = zip(*dataset)
        X = np.array(x_values)
        Y = np.array(y_values)
        plt.plot(X, Y, 'o', label=dataset_path, markersize=2)
    plt.legend()
    plt.show(block=True)
