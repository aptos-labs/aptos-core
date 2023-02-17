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
    if x<190: return None
    jsonstr = Path(f'{path}/new/estimates.json').read_text()
    data = json.loads(jsonstr)
    y = data['median']['point_estimate']
    assert type(y)==float
    return x,y

def rand_dataset(bench_name):
    sampled_data_points = [load_data_point(path) for path in glob(f'target/criterion/{bench_name}/*')]
    sampled_data_points = [p for p in sampled_data_points if p!=None]
    sampled_data_points.sort()
    return sampled_data_points

def load_and_linear_regression(bench_name):
    data_points = rand_dataset(bench_name)
    x_values, y_values = zip(*data_points)
    x_values = np.array(x_values)
    y_values = np.array(y_values)
    A = np.vstack([x_values, np.ones(len(x_values))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, y_values, rcond=None)
    return x_values, y_values, sol, residuals

def main(bench_name, gas_per_ns, should_also_plot):
    x, y, sol, residuals = load_and_linear_regression(bench_name)
    k,b = sol
    if should_also_plot:
        plt.plot(x, y, 'o', label='Original data', markersize=10)
        plt.plot(x, k*x+b, 'r', label='Fitted line')
        plt.legend()
        plt.show(block=True)
    k=k*gas_per_ns
    b=b*gas_per_ns
    return k,b

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--gas_per_ns', required=True, type=float)
    parser.add_argument('-plot', action='store_true')
    args = parser.parse_args()
    k,b = main('ristretto255/vartime_multiscalar_mul', args.gas_per_ns, args.plot)
    data = {'k': k, 'b': b}
    print(json.dumps(data))
