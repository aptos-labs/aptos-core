import argparse
import subprocess
from glob import glob
import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from pprint import pprint
import random
import re

benches = glob('target/criterion/blst/g1_affine_msm_size_*')

def invoke(cmd, **kwargs):
    print()
    print('================================INVOKE================================')
    print(cmd)
    proc = subprocess.run(cmd, shell=True, capture_output=True)
    print('================================STDOUT================================')
    print(proc.stdout.decode('utf-8'))
    print('================================STDERR================================')
    print(proc.stderr.decode('utf-8'))
    print('======================================================================')
    print()
    return proc

def parse_bench_data(bench):
    x = int(re.match(r'target/criterion/blst/g1_affine_msm_size_(\d+)', bench).group(1))
    data = np.genfromtxt(
        f'{bench}/new/raw.csv',
        delimiter=",",
        dtype=None,
        names=True,
        encoding=None
    )
    return (x,data)

data_of_bench = dict([parse_bench_data(bench) for bench in benches])

def rand_bench_result(bench_results):
    random_row = random.choice(bench_results)
    time_per_iteration = random_row['sample_measured_value'] / random_row['iteration_count']
    return time_per_iteration

def rand_dataset():
    sampled_data_points = [(num,rand_bench_result(bench_results)) for num,bench_results in data_of_bench.items()]
    sampled_data_points.sort()
    return sampled_data_points

def rand_linear_solution():
    data_points = rand_dataset()
    x_values, y_values = zip(*data_points)
    x_values = np.array(x_values)
    y_values = np.array(y_values)
    A = np.vstack([x_values, np.ones(len(x_values))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, y_values, rcond=None)
    return x_values, y_values, sol, residuals

def get_x(bench_result_path):
    x = int(re.match(r'target/criterion/.*/(\d+)/new/raw.csv', bench_result_path).group(1))
    return x

def get_values(bench_result_path):
    rows = np.genfromtxt(
        bench_result_path,
        delimiter=",",
        dtype=None,
        names=True,
        encoding=None
    )
    values = [row['sample_measured_value'] / row['iteration_count'] for row in rows]
    return values

SHA256_SAMPLES = None

def get_sha256_samples():
    global SHA256_SAMPLES
    if SHA256_SAMPLES == None:
        SHA256_SAMPLES = reload_sha256_samples()
    return SHA256_SAMPLES

def reload_sha256_samples():
    bench_result_paths = glob('target/criterion/hash/SHA2-256/*/new/raw.csv')
    ret = {get_x(b):get_values(b) for b in bench_result_paths}
    return ret

def rand_sha256_dataset():
    return sorted([(x,random.choice(y_values)) for (x,y_values) in get_sha256_samples().items()])

def rand_sha256_solution():
    data_points = rand_sha256_dataset()
    x_values, y_values = zip(*data_points)
    x_values = np.array(x_values)
    y_values = np.array(y_values)
    A = np.vstack([x_values, np.ones(len(x_values))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, y_values, rcond=None)
    return x_values, y_values, sol, residuals

def rand_sha256_chart():
    x, y, sol, residuals = rand_sha256_solution()
    k,b = sol
    print(f'k={k}')
    print(f'b={b}')
    plt.plot(x, y, 'o', label='Original data', markersize=10)
    plt.plot(x, k*x + b, 'r', label='Fitted line')
    plt.legend()
    plt.show(block=True)

rand_sha256_chart()

# num_rounds = 1000
#
# if __name__=='__main__':
#     parser = argparse.ArgumentParser()
#     parser.add_argument('action', choices=['show_rand_sol', '1000_sample_avg_residual'])
#     args = parser.parse_args()
#     if args.action=='show_rand_sol':
#         x, y, sol, residuals = globals()[f'rand_linear_solution']()
#         m,c = sol
#         plt.plot(x, y, 'o', label='Original data', markersize=10)
#         plt.plot(x, m*x + c, 'r', label='Fitted line')
#         plt.legend()
#         plt.show(block=True)
#     elif args.action=='1000_sample_avg_residual':
#         avg = sum(globals()[f'rand_linear_solution']()[3] for _ in range(num_rounds))/num_rounds
#         print(avg)
#     else:
#         assert False
