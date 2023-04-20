#!/usr/bin/env python3

'''
This module automated the steps to
calculate gas parameters for `algebra.move` natives from benchmarking results,
then update the gas parameter definitions in rust.
'''

import argparse
import fit_linear_model
import load_bench_ns
import load_bench_datapoints
from math import ceil
from pathlib import Path
from time import time

# Typically you are making a new version of gas schedule,
# so this should be larger than `LATEST_GAS_FEATURE_VERSION` in `aptos-move/aptos-gas/src/gas_meter.rs`.
TARGET_GAS_VERSION = 8

def get_bench_ns_linear(bench_path):
    datapoints = load_bench_datapoints.main(bench_path)
    X,Y,k,b = fit_linear_model.main(datapoints)
    return X,Y,k,b

def prettify_number(x:int) -> str:
    s = str(x)
    n = len(s)
    b = n % 3
    chunks_0 = [s[:b]] if b>=1 else []
    chunks = chunks_0 + [s[i:i+3] for i in range(b,n,3)]
    return '_'.join(chunks)

def get_algebra_lines(gas_per_ns):
    nanoseconds = {}
    nanoseconds['ark_bls12_381_fr_add'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_add')
    nanoseconds['ark_bls12_381_fr_deser'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_deser')
    nanoseconds['ark_bls12_381_fr_div'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_div')
    nanoseconds['ark_bls12_381_fr_eq'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_eq')
    nanoseconds['ark_bls12_381_fr_from_u64'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_from_u64')
    nanoseconds['ark_bls12_381_fr_inv'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_inv')
    nanoseconds['ark_bls12_381_fr_mul'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_mul')
    nanoseconds['ark_bls12_381_fr_neg'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_neg')
    nanoseconds['ark_bls12_381_fr_one'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_one')
    nanoseconds['ark_bls12_381_fr_serialize'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_serialize')
    nanoseconds['ark_bls12_381_fr_square'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_square')
    nanoseconds['ark_bls12_381_fr_sub'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_sub')
    nanoseconds['ark_bls12_381_fr_zero'] = load_bench_ns.main('target/criterion/ark_bls12_381/fr_zero')
    nanoseconds['ark_bls12_381_fq12_add'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_add')
    nanoseconds['ark_bls12_381_fq12_clone'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_clone')
    nanoseconds['ark_bls12_381_fq12_deser'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_deser')
    nanoseconds['ark_bls12_381_fq12_div'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_div')
    nanoseconds['ark_bls12_381_fq12_eq'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_eq')
    nanoseconds['ark_bls12_381_fq12_from_u64'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_from_u64')
    nanoseconds['ark_bls12_381_fq12_inv'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_inv')
    nanoseconds['ark_bls12_381_fq12_mul'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_mul')
    nanoseconds['ark_bls12_381_fq12_neg'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_neg')
    nanoseconds['ark_bls12_381_fq12_one'] = 1
    nanoseconds['ark_bls12_381_fq12_pow_u256'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_pow_u256')
    nanoseconds['ark_bls12_381_fq12_serialize'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_serialize')
    nanoseconds['ark_bls12_381_fq12_square'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_square')
    nanoseconds['ark_bls12_381_fq12_sub'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_sub')
    nanoseconds['ark_bls12_381_fq12_zero'] = load_bench_ns.main('target/criterion/ark_bls12_381/fq12_zero')
    nanoseconds['ark_bls12_381_g1_affine_deser_comp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_affine_deser_comp')
    nanoseconds['ark_bls12_381_g1_affine_deser_uncomp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_affine_deser_uncomp')
    nanoseconds['ark_bls12_381_g1_affine_serialize_comp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_affine_serialize_comp')
    nanoseconds['ark_bls12_381_g1_affine_serialize_uncomp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_affine_serialize_uncomp')
    nanoseconds['ark_bls12_381_g1_proj_add'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_add')
    nanoseconds['ark_bls12_381_g1_proj_double'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_double')
    nanoseconds['ark_bls12_381_g1_proj_eq'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_eq')
    nanoseconds['ark_bls12_381_g1_proj_generator'] = 1
    nanoseconds['ark_bls12_381_g1_proj_infinity'] = 1
    nanoseconds['ark_bls12_381_g1_proj_neg'] = 1
    nanoseconds['ark_bls12_381_g1_proj_scalar_mul'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_scalar_mul')
    nanoseconds['ark_bls12_381_g1_proj_sub'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_sub')
    nanoseconds['ark_bls12_381_g1_proj_to_affine'] = load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_to_affine')
    nanoseconds['ark_bls12_381_g2_affine_deser_comp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_affine_deser_comp')
    nanoseconds['ark_bls12_381_g2_affine_deser_uncomp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_affine_deser_uncomp')
    nanoseconds['ark_bls12_381_g2_affine_serialize_comp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_affine_serialize_comp')
    nanoseconds['ark_bls12_381_g2_affine_serialize_uncomp'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_affine_serialize_uncomp')
    nanoseconds['ark_bls12_381_g2_proj_add'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_add')
    nanoseconds['ark_bls12_381_g2_proj_double'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_double')
    nanoseconds['ark_bls12_381_g2_proj_eq'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_eq')
    nanoseconds['ark_bls12_381_g2_proj_generator'] = 1
    nanoseconds['ark_bls12_381_g2_proj_infinity'] = 1
    nanoseconds['ark_bls12_381_g2_proj_neg'] = 1
    nanoseconds['ark_bls12_381_g2_proj_scalar_mul'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_scalar_mul')
    nanoseconds['ark_bls12_381_g2_proj_sub'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_sub')
    nanoseconds['ark_bls12_381_g2_proj_to_affine'] = load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_to_affine')
    nanoseconds['ark_bls12_381_pairing'] = load_bench_ns.main('target/criterion/ark_bls12_381/pairing')
    _,_,nanoseconds['ark_bls12_381_multi_pairing_per_pair'],nanoseconds['ark_bls12_381_multi_pairing_base'] = get_bench_ns_linear('target/criterion/ark_bls12_381/pairing_product')
    _,_,nanoseconds['ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte'],nanoseconds['ark_h2c_bls12381g1_xmd_sha256_sswu_base'] = get_bench_ns_linear('target/criterion/ark_bls12_381/hash_to_g1_proj')
    _,_,nanoseconds['ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte'],nanoseconds['ark_h2c_bls12381g2_xmd_sha256_sswu_base'] = get_bench_ns_linear('target/criterion/ark_bls12_381/hash_to_g2_proj')
    gas_units = {k:gas_per_ns*v for k,v in nanoseconds.items()}
    lines = [f'    [.algebra.{k}, {{ {TARGET_GAS_VERSION}.. => "algebra.{k}" }}, {prettify_number(v)} * MUL],' for k,v in sorted(gas_units.items())]
    return lines

def main(gas_per_ns):
    path = Path('aptos-move/aptos-gas/src/aptos_framework.rs')
    lines = path.read_text().split('\n')
    line_id_begin = lines.index('    // Algebra gas parameters begin.')
    line_id_end = lines.index('    // Algebra gas parameters end.')
    generator_note_line = f'    // Generated at time {time()} by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns={gas_per_ns}.'
    new_lines = lines[:line_id_begin+1] + [generator_note_line] + get_algebra_lines(gas_per_ns) + lines[line_id_end:]
    path.write_text('\n'.join(new_lines))

if __name__=='__main__':
    parser = argparse.ArgumentParser(
        description='Generate gas parameters for algebra module in `aptos-move/aptos-gas/src/aptos_framework.rs`.')
    parser.add_argument('--gas_per_ns', required=True, type=float)
    args = parser.parse_args()
    main(args.gas_per_ns)
