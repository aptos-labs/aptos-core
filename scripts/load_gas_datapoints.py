import argparse
import json
import load_us_datapoints
import numpy as np
from pathlib import Path

def main(bench_name, gas_per_us):
    data_points = load_us_datapoints.main(bench_name)
    x_values, y_values = zip(*data_points)
    gas_y_values = [gas_per_us*y for y in y_values]
    x_values = np.array(x_values)
    gas_y_values = np.array(gas_y_values)
    return x_values, gas_y_values

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_name', required=True)
    parser.add_argument('--gas_per_us', type=float, required=True)
    args = parser.parse_args()
    x,y = main(args.bench_name, args.gas_per_us)
    print({'x_values':x, 'y_values':y})
