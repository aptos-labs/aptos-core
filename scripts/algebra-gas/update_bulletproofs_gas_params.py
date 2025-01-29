#!/usr/bin/env python3

'''
This module automated the steps to
calculate gas parameters for `bulletproofs.move` natives from benchmarking results,
then update the gas parameter definitions in rust.
'''

import argparse
import fit_linear_model
import load_bench_ns
import load_bench_datapoints
from math import ceil
from pathlib import Path
from time import time
import math

# Typically you are making a new version of gas schedule,
# so this should be larger than `LATEST_GAS_FEATURE_VERSION` in `aptos-move/aptos-gas/src/gas_meter.rs`.
TARGET_GAS_VERSION = 11

def get_bench_ns_linear(bench_path):
    datapoints = load_bench_datapoints.main(bench_path)
    X,Y,k,b = fit_linear_model.main(datapoints)
    return X,Y,k,b

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
        _,_,verify_slope,verify_base = get_bench_ns_linear(f'target/criterion/bulletproofs/range_verify_batch_{batch_size}')
        _,_,deserialize_slope,deserialize_base = get_bench_ns_linear(f'target/criterion/bulletproofs/range_proof_deserialize_batch_{batch_size}')

        nanoseconds[f'bulletproofs_verify_base_{batch_size}'] = verify_base
        nanoseconds[f'bulletproofs_verify_per_bit_{batch_size}'] = verify_slope
        nanoseconds[f'bulletproofs_deserialize_base_{batch_size}'] = deserialize_base
        nanoseconds[f'bulletproofs_deserialize_per_byte_{batch_size}'] = deserialize_slope

    gas_units = {k:gas_per_ns*v for k,v in nanoseconds.items()}

    lines = []

    for batch_size in [1, 2, 4, 8, 16]:
        lines.append(f'        [bulletproofs_verify_base_{batch_size}: InternalGas, {{ {TARGET_GAS_VERSION}.. => "bulletproofs.verify.base_{batch_size}" }}, {prettify_number(gas_units[f"bulletproofs_verify_base_{batch_size}"])}],')

    for batch_size in [1, 2, 4, 8, 16]:
        lines.append(f'        [bulletproofs_verify_per_bit_{batch_size}: InternalGasPerArg, {{ {TARGET_GAS_VERSION}.. => "bulletproofs.verify.per_bit_{batch_size}" }}, {prettify_number(gas_units[f"bulletproofs_verify_per_bit_{batch_size}"])}],')

    for batch_size in [1, 2, 4, 8, 16]:
        lines.append(f'        [bulletproofs_deserialize_base_{batch_size}: InternalGas, {{ {TARGET_GAS_VERSION}.. => "bulletproofs.deserialize.base_{batch_size}" }}, {prettify_number(gas_units[f"bulletproofs_deserialize_base_{batch_size}"])}],')

    for batch_size in [1, 2, 4, 8, 16]:
        lines.append(f'        [bulletproofs_deserialize_per_byte_{batch_size}: InternalGasPerByte, {{ {TARGET_GAS_VERSION}.. => "bulletproofs.deserialize.per_byte_{batch_size}" }}, {prettify_number(gas_units[f"bulletproofs_deserialize_per_byte_{batch_size}"])}],')

    return lines

def main(gas_per_ns):
    path = Path('aptos-move/aptos-gas-schedule/src/gas_schedule/aptos_framework.rs')
    lines = path.read_text().split('\n')
    line_id_begin = lines.index('        // Bulletproofs gas parameters begin.')
    line_id_end = lines.index('        // Bulletproofs gas parameters end.')
    generator_note_line = f'        // Generated at time {time()} by `scripts/algebra-gas/update_bulletproofs_gas_params.py` with gas_per_ns={gas_per_ns}.'
    new_lines = lines[:line_id_begin+1] + [generator_note_line] + get_bulletproofs_lines(gas_per_ns) + lines[line_id_end:]
    path.write_text('\n'.join(new_lines))

if __name__=='__main__':
    parser = argparse.ArgumentParser(
        description='Generate gas parameters for bulletproofs module in `aptos-move/aptos-gas/src/aptos_framework.rs`.')
    parser.add_argument('--gas_per_ns', required=True, type=float)
    args = parser.parse_args()
    main(args.gas_per_ns)
