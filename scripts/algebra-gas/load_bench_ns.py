#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

def main(bench_path):
    '''Parse a value from a given benchmark result.

    Param `bench_path` can be the result of any single-datapoint bench (e.g., 'target/criterion/ark_bls12_381/fr_add'),
    or a single result of a serial bench (e.g., 'target/criterion/hash/SHA2-256/0').
    '''
    json_path = Path(f'{bench_path}/new/estimates.json')
    if not json_path.exists(): return None
    jsonstr = json_path.read_text()
    obj = json.loads(jsonstr)
    ns = obj['median']['point_estimate']
    assert type(ns)==float
    return ns

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bench_path', required=True)
    args = parser.parse_args()
    ns = main(args.bench_path)
    print(json.dumps({'ns':ns}))
