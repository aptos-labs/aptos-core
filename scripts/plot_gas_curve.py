import argparse
import json
import load_gas_datapoints
import matplotlib.pyplot as plt

def percentile(values, k):
    values_sorted = sorted(values)
    n = len(values)
    idx = int(n*k)
    return values_sorted[idx]


def main(bench_name, gas_k, gas_b, gas_per_us):
    x, y_consumed = load_gas_datapoints.main(bench_name, gas_per_us)
    n = len(x)
    y_charged = [gas_k*x+gas_b for x in x]
    charge_rates = [y_charged[i]/y_consumed[i] for i in range(n)]
    plt.plot(x, y_consumed, 'o', label='gas consumed', markersize=1)
    plt.plot(x, y_charged, 'r', label='gas charged')
    plt.legend()
    plt.show(block=True)
    return {
        'charge_rate_p99': percentile(charge_rates, 0.99),
        'charge_rate_p90': percentile(charge_rates, 0.95),
        'charge_rate_p10': percentile(charge_rates, 0.10),
        'charge_rate_p01': percentile(charge_rates, 0.01),
    }

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_name', required=True)
    parser.add_argument('--gas_k', type=float, required=True)
    parser.add_argument('--gas_b', type=float, required=True)
    parser.add_argument('--gas_per_us', required=True, type=float)
    args = parser.parse_args()
    result = main(args.bench_name, args.gas_k, args.gas_b, args.gas_per_us)
    print(json.dumps(result, indent=4))
