#!/usr/bin/env python3

'''
This module automated the steps to
calculate gas parameters for `bulletproofs.move` batch verification natives from benchmarking results,
then update the gas parameter definitions in rust.
'''

import argparse
import load_bench_ns
from pathlib import Path
from time import time
import math

# Typically you are making a new version of gas schedule,
# so this should be larger than `LATEST_GAS_FEATURE_VERSION` in `velor-move/velor-gas/src/gas_meter.rs`.
TARGET_GAS_VERSION = 'RELEASE_V1_28'

def prettify_number(x:int) -> str:
    s = str(math.ceil(x))
    n = len(s)
    b = n % 3
    chunks_0 = [s[:b]] if b>=1 else []
    chunks = chunks_0 + [s[i:i+3] for i in range(b,n,3)]
    return '_'.join(chunks)

def get_bulletproofs_lines(gas_per_ns):
    nanoseconds = {}

    for batch_size in [1, 2, 4, 8, 16]:
        for num_bits in [8, 16, 32, 64]:
            ns = load_bench_ns.main(f'target/criterion/bulletproofs_batch_verify/range_verify/batch={batch_size}_bits={num_bits}')

            nanoseconds[f'bulletproofs_verify_base_batch_{batch_size}_bits_{num_bits}'] = ns

    gas_units = {k:gas_per_ns*v for k,v in nanoseconds.items()}

    lines = []

    for batch_size in [1, 2, 4, 8, 16]:
        for num_bits in [8, 16, 32, 64]:
            lines.append(f'        [bulletproofs_verify_base_batch_{batch_size}_bits_{num_bits}: InternalGas, {{ {TARGET_GAS_VERSION}.. => "bulletproofs.verify.base_batch_{batch_size}_bits_{num_bits}" }}, {prettify_number(gas_units[f"bulletproofs_verify_base_batch_{batch_size}_bits_{num_bits}"])}],')

    return lines

def main(gas_per_ns):
    path = Path('velor-move/velor-gas-schedule/src/gas_schedule/velor_framework.rs')
    lines = path.read_text().split('\n')
    line_id_begin = lines.index('        // Bulletproofs batch verify gas parameters begin.')
    line_id_end = lines.index('        // Bulletproofs batch verify gas parameters end.')
    generator_note_line = f'        // Generated at time {time()} by `scripts/algebra-gas/update_bulletproofs_batch_verify_gas_params.py` with gas_per_ns={gas_per_ns}.'
    new_lines = lines[:line_id_begin+1] + [generator_note_line] + get_bulletproofs_lines(gas_per_ns) + lines[line_id_end:]
    path.write_text('\n'.join(new_lines))

if __name__=='__main__':
    parser = argparse.ArgumentParser(
        description='Generate gas parameters for bulletproofs batch verification in `velor-move/velor-gas-schedule/src/gas_schedule/velor_framework.rs`.')
    parser.add_argument('--gas_per_ns', required=True, type=float)
    args = parser.parse_args()
    main(args.gas_per_ns)
