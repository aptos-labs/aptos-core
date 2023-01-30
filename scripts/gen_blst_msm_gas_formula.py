import argparse
from glob import glob
from math import log2
import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from pprint import pprint
import random
import re

benches = glob('target/criterion/blst/g1_affine_msm_size_*')

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

def rand_q_over_log_q_solution():
    data_points = rand_dataset()
    x_values, y_values = zip(*data_points)
    x_values = [x/log2(x) for x in x_values]
    x_values = np.array(x_values)
    y_values = np.array(y_values)
    A = np.vstack([x_values, np.ones(len(x_values))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, y_values, rcond=None)
    return x_values, y_values, sol, residuals

num_rounds = 1000

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('model', choices=['linear', 'q_over_log_q'])
    parser.add_argument('action', choices=['show_rand_sol', '1000_sample_avg_residual'])
    args = parser.parse_args()
    if args.action=='show_rand_sol':
        x, y, sol, residuals = globals()[f'rand_{args.model}_solution']()
        m,c = sol
        plt.plot(x, y, 'o', label='Original data', markersize=10)
        plt.plot(x, m*x + c, 'r', label='Fitted line')
        plt.legend()
        plt.show(block=True)
    elif args.action=='1000_sample_avg_residual':
        avg = sum(globals()[f'rand_{args.model}_solution']()[3] for _ in range(num_rounds))/num_rounds
        print(avg)
    else:
        assert False
