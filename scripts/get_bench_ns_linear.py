import argparse
import get_bench_ns
from glob import glob
import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

CRITERION_ROOT = Path('target/criterion')

def get_datapoint(path):
    sub_bench_path = str(Path(path).relative_to(CRITERION_ROOT))
    items = sub_bench_path.split('/')
    if not items[-1].isdecimal(): return None
    arg = int(items[-1])
    ns = get_bench_ns.main(sub_bench_path)
    return (arg,ns)

def main(bench_path):
    datapoints = [get_datapoint(path) for path in glob(f'{CRITERION_ROOT}/{bench_path}/*')]
    datapoints = [dp for dp in datapoints if dp!=None]
    assert len(datapoints)>=1
    x_values, y_values = zip(*datapoints)
    X = np.array(x_values)
    Y = np.array(y_values)
    A = np.vstack([X, np.ones(len(X))]).T
    sol, residuals, _, _ = np.linalg.lstsq(A, Y, rcond=None)
    k,b = sol
    return (X, Y, k, b)

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_path', required=True)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    X,Y,k,b = main(args.bench_path)
    print(json.dumps({'b':b, 'k':k}))
    if args.plot:
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.plot(X, k*X+b, 'r', label='ns fitted')
        plt.legend()
        plt.show(block=True)
