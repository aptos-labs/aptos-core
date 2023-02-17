import argparse
import json
import load_gas_datapoints
import matplotlib.pyplot as plt

def f(x):
    if x<190:
        return 115435.3532982281*x+413486.1099769859
    else:
        return 68077.02033089707*x+18827193.13818383

def percentile(values, k):
    values_sorted = sorted(values)
    n = len(values)
    idx = int(n*k)
    return values_sorted[idx]

def main(gas_per_us):
    x_values, y_consumed = load_gas_datapoints.main('ristretto255/vartime_multiscalar_mul', gas_per_us)
    n = len(x_values)
    y_charged = [f(x) for x in x_values]
    charge_rate_x_pairs = [(y_charged[i]/y_consumed[i], i) for i in range(n)]
    plt.plot(x_values, y_consumed, 'o', label='gas consumed', markersize=1)
    plt.plot(x_values, y_charged, 'r', label='gas charged')
    plt.legend()
    plt.show(block=True)
    return {
        'charge_rate_p99': percentile(charge_rate_x_pairs, 0.99)[0],
        'charge_rate_p90': percentile(charge_rate_x_pairs, 0.95)[0],
        'charge_rate_p10': percentile(charge_rate_x_pairs, 0.10)[0],
        'charge_rate_p01': percentile(charge_rate_x_pairs, 0.01)[0],
    }

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--gas_per_us', required=True, type=float)
    args = parser.parse_args()
    result = main(args.gas_per_us)
    print(json.dumps(result, indent=4))
