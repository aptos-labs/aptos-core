import argparse
from glob import glob
import json
# import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import re

def load_data_point(path):
    match = re.match(r'.*/(\d+)$', path)
    if match==None: return None
    x = int(match.group(1))
    jsonstr = Path(f'{path}/new/estimates.json').read_text()
    data = json.loads(jsonstr)
    y = data['median']['point_estimate']
    assert type(y)==float
    return x,y

def load_ns_by_x_datapoints(bench_name):
    sampled_data_points = [load_data_point(path) for path in glob(f'target/criterion/{bench_name}/*')]
    sampled_data_points = [p for p in sampled_data_points if p!=None]
    sampled_data_points.sort()
    return sampled_data_points

def load_gas_by_x_datapoints(bench_name, gas_per_ns):
    data_points = load_ns_by_x_datapoints(bench_name)
    x_values, y_values = zip(*data_points)
    gas_y_values = [gas_per_ns*y for y in y_values]
    x_values = np.array(x_values)
    gas_y_values = np.array(gas_y_values)
    return x_values, gas_y_values

def main(bench_name, gas_k, gas_b, gas_per_ns):
    x, y = load_gas_by_x_datapoints(bench_name, gas_per_ns)
    plt.plot(x, y, 'o', label='gas consumed', markersize=1)
    plt.plot(x, gas_k*x+gas_b, 'r', label='gas charged')
    plt.legend()
    plt.show(block=True)

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_name', required=True)
    parser.add_argument('--gas_k', type=float, required=True)
    parser.add_argument('--gas_b', type=float, required=True)
    parser.add_argument('--gas_per_ns', required=True, type=float)
    args = parser.parse_args()
    k,b = main(args.bench_name, args.gas_k, args.gas_b, args.gas_per_ns)
