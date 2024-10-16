#!/bin/bash

set -e

if [ ! -e ".circuit-root" ]; then
  echo "Error: the current working directory is not circuit package root (the directory that contains `.circuit-root`)"
  exit 1
fi

print_usage() {
cat <<DONE
Usage: `basename $0` <output-dir>
DONE
}

pushd() {
    command pushd "$@" > /dev/null
}

popd() {
    command popd "$@" > /dev/null
}

trusted_setup() {
    outdir=$1

    mkdir -p $outdir
    outdir=`realpath $outdir`

    echo
    echo "Generating proving key and verification key in $outdir"

    ptau_repo=aptos-keyless-trusted-setup-contributions-may-2024
    ptau_file=powersOfTau28_hez_final_21.ptau
    if [ ! -d "$ptau_repo" ]; then
        echo
        echo "You haven't downloaded the .ptau file yet. Downloading now..."
        GIT_LFS_SKIP_SMUDGE=1 git clone git@github.com:aptos-labs/$ptau_repo.git
        pushd $ptau_repo/
        {
            echo
            echo "Downloading ~2.4 GiB file. This will take a while..."
            git lfs pull --include $ptau_file
            echo "Done downloading .ptau file."
        }
        popd
    fi
    ptaudir=`realpath $ptau_repo`

    pushd templates/
    {
        echo
        echo "Re-compiling circuit. This will take several seconds..."
        circom -l . -l $(npm root -g) main.circom --r1cs --wasm --sym

        rm -f $outdir/prover_key.zkey
        rm -f $outdir/verification_key.json

        echo
        echo "Running dummy phase-2 setup..."
        echo "This will take several minutes..."
        time snarkjs groth16 setup main.r1cs $ptaudir/$ptau_file $outdir/prover_key.zkey

        echo
        echo "Exporting verification key..."
        snarkjs zkey export verificationkey $outdir/prover_key.zkey $outdir/verification_key.json
    }
    popd

    echo
    echo "Done. Find the prover_key.zkey and verification_key.json files in $outdir"
}

if [ "$#" -ne 1 ]; then
    print_usage
    exit 2
fi

if [ -e "$1" ]; then
  echo "Error: output path already exists."
  exit 3
fi

trusted_setup "$1"
