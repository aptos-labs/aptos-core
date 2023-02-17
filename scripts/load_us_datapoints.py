import argparse
from glob import glob
import json
import load_data_point

def main(bench_name):
    sampled_data_points = [load_data_point.main(path) for path in glob(f'target/criterion/{bench_name}/*')]
    sampled_data_points = [p for p in sampled_data_points if p!=None]
    sampled_data_points.sort()
    return sampled_data_points

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_name', required=True)
    args = parser.parse_args()
    result = main(args.bench_name)
    print(json.dumps(result, indent=4))
