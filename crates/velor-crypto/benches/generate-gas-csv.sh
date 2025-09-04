#!/bin/bash

scriptdir=$(cd $(dirname $0); pwd -P)

repo_root=$(readlink -f $scriptdir/../../../)

#echo "Repo root: $repo_root"

critdir=$repo_root/target/criterion/

if [ $# -ne 1 ]; then
    echo "Usage: $0 <output-directory>"
    echo
    echo "A bunch of .csv files will be created in <output-directory>"
    exit 1
fi

outdir=$1
mkdir -p $outdir
echo "Output directory: $outdir"

#
# Variable-size input operations
# (Currently, just hashing)
#

[ ! -d $critdir/hash/ ] && { echo "No benchmark results for $hash_func. Be sure to run 'cargo bench -- hash' first..."; exit 1; }
[ ! -f $scriptdir/parse-hash-benches.sh ] && { echo "Expected a '$scriptdir/parse-hash-benches.sh' executable file."; exit 1; }

out_hash=$outdir/hash

echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-sha2-256.csv
$scriptdir/parse-hash-benches.sh SHA2-256 ${out_hash}-sha2-256.csv
echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-sha2-512.csv
$scriptdir/parse-hash-benches.sh SHA2-512 ${out_hash}-sha2-512.csv
echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-sha3-256.csv
$scriptdir/parse-hash-benches.sh SHA3-256 ${out_hash}-sha3-256.csv
echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-keccak-256.csv
$scriptdir/parse-hash-benches.sh Keccak-256 ${out_hash}-keccak-256.csv
echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-bls12381_g1.csv
$scriptdir/parse-hash-benches.sh hash_to_bls12381_g1 ${out_hash}-bls12381_g1.csv
echo "hash_function,input_size,time_in_nanosecs" >${out_hash}-bls12381_g2.csv
$scriptdir/parse-hash-benches.sh hash_to_bls12381_g2 ${out_hash}-bls12381_g2.csv

get_mean() {
  file=$1

  mean=`cat "$file" | jq .slope.point_estimate`
  if [ "$mean" = "null" ]; then
      #echo "No .slope.point_estimate, using .mean.point_estimate" 1>&2

      mean=`cat "$file" | jq .mean.point_estimate`
  fi

  echo $mean
}

#
# Fixed-size input operations
#
operations="
bls12381:pk_deserialize
bls12381:aggregate_pks/1024
bls12381:pk_prime_order_subgroup_check
bls12381:sig_deserialize
bls12381:aggregate_sigshare/1024
bls12381:sig_prime_order_subgroup_check
bls12381:NOOP.per_sig_verify
bls12381:pop_verify
bls12381:NOOP.per_pairing

ed25519:pk_deserialize
ed25519:small_subgroup_check
ed25519:sig_deserialize
ed25519:sig_verify_zero_bytes

secp256k1:ecdsa_recover

ristretto255:basepoint_mul
ristretto255:basepoint_double_mul
ristretto255:point_add
ristretto255:NOOP.point_clone
ristretto255:point_compress
ristretto255:point_decompress
ristretto255:point_equals
ristretto255:point_from_64_uniform_bytes
ristretto255:point_identity
ristretto255:point_mul
ristretto255:point_neg
ristretto255:point_sub
ristretto255:NOOP.point_parse_arg
ristretto255:scalar_add
ristretto255:scalar_reduced_from_32_bytes
ristretto255:scalar_uniform_from_64_bytes
ristretto255:scalar_from_u128
ristretto255:scalar_from_u64
ristretto255:scalar_invert
ristretto255:scalar_is_canonical
ristretto255:scalar_mul
ristretto255:scalar_neg
ristretto255:scalar_sub
ristretto255:NOOP.scalar_parse_arg
"

while read -r module_and_op; do
  module=`echo "$module_and_op" | cut -f 1 -d':'`
  op=`echo "$module_and_op" | cut -f 2   -d':'`
  out_file=$outdir/${module}.csv


  # We leave spaces sometimes for ease of reading
  if [ "$op" = "" ]; then
      #echo "Skipping empty line..."
      continue
  fi

  echo

  # We use no-ops in the CSV file to leave a gap in case we will add a field there later, so that line offsets of other
  # fields don't change and mess up our spreadsheet calculation.
  if echo "$op" | grep "NOOP\..*" >/dev/null; then
      noop=`echo "$op" | cut -f 2 -d'.'`
      echo "Skipping $module no-op for $noop..."
      echo $module,$noop,NA >>$out_file
      continue
  fi

  echo "$module module, $op operation"

  # Add the CSV headers in on the first iteration
  if [ ! -f $out_file ]; then
      echo "module,operation_name,time_in_nanosecs" >$out_file
  fi

  # Make sure benchmarks were run
  [ ! -d "$critdir/$module/$op/" ] && {
    echo "No benchmark results for '$module/$op'. Be sure to run 'cargo bench -- $module/$op' first...";
    echo "$op,NA" >>$out_file
    continue;
  }

  mean=`get_mean "$critdir/$module/$op/new/estimates.json"`

  # For some functions that take an input of size n, we pick a reasonably large batch size n and estimate their per-input
  # cost by dividing the total time by n.
  if echo "$op" | grep ".*/.*" >/dev/null; then
      op_name=`echo "$op" | cut -f 1 -d'/'`
      batch_size=`echo "$op" | cut -f 2 -d'/'`

      echo "Batched operation: $op_name ($batch_size)"

      mean=`echo $mean / 1024.0 | bc -l`
  fi

  echo "$module,$op,$mean" >>$out_file
done <<< "$operations"

echo
echo "Exited gracefully."
echo

