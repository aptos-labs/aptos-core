import argparse
import json
from pathlib import Path

def main(bench_path):
    jsonstr = Path(f'target/criterion/{bench_path}/new/estimates.json').read_text()
    obj = json.loads(jsonstr)
    ns = obj['median']['point_estimate']
    assert type(ns)==float
    return ns

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_path', required=True, help='Example: ark_bls12_381/fr_add')
    args = parser.parse_args()
    ns = main(args.bench_path)
    print(json.dumps({'ns':ns}))
