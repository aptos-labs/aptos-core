#!/usr/bin/env python3

import load_bench_datapoints
import matplotlib.pyplot as plt

def main(bench_path, model_path, plot):
    datapoints = load_bench_datapoints.main(bench_path)
    x_values, y_values = zip(*datapoints)
    X = np.array(x_values)
    Y = np.array(y_values)
    Y_hat = model(X)
    model = load_model.main(model_path)
    if plot:
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.plot(X, model(X), 'r', label='ns predicted')
        plt.legend()
        plt.show(block=True)
