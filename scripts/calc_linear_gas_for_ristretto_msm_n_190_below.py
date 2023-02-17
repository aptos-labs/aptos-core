import argparse
from glob import glob
import json
import load_data_point
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import re

def load_us_datapoints(bench_name):
    sampled_data_points = [load_data_point.main(path) for path in glob(f'target/criterion/{bench_name}/*')]
    sampled_data_points = [p for p in sampled_data_points if p!=None and p[0]<190]
    sampled_data_points.sort()
    return sampled_data_points

def load_gas_datapoints_and_linear_regression(bench_name, gas_per_us):
    data_points = load_us_datapoints(bench_name)
    x_values, y_values = zip(*data_points)
    gas_y_values = [y*gas_per_us for y in y_values]
    x_values = np.array(x_values)
    gas_y_values = np.array(gas_y_values)
    A = np.vstack([x_values, np.ones(len(x_values))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, gas_y_values, rcond=None)
    return x_values, gas_y_values, sol, residuals

def main(bench_name, gas_per_us, should_also_plot):
    x, y, sol, residuals = load_gas_datapoints_and_linear_regression(bench_name, gas_per_us)
    k,b = sol
    if should_also_plot:
        plt.plot(x, y, 'o', label='gas consumed', markersize=2)
        plt.plot(x, k*x+b, 'r', label='gas charged')
        plt.legend()
        plt.show(block=True)
    return k,b

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--gas_per_us', required=True, type=float)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    k,b = main('ristretto255/vartime_multiscalar_mul', args.gas_per_us, args.plot)
    data = {'k': k, 'b': b}
    print(json.dumps(data))
