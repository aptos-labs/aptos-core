#!/bin/bash

set -e

scriptdir="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"
repodir=$scriptdir

echo "Executing from directory: $scriptdir"

print_usage() {
cat <<DONE
Usage: `basename $0` <keyless-circuit-branch> <output-dir>
DONE
}

pushd() {
    command pushd "$@" > /dev/null
}

popd() {
    command popd "$@" > /dev/null
}

trusted_setup() {
    branch=$1
    outdir=$2

    mkdir -p $outdir
    outdir=`realpath $outdir`

    echo
    echo "Generating proving key and verification key in $outdir"

    tmpdir=tmp
    mkdir -p $tmpdir
    pushd $tmpdir/ 
    {
        ptau_repo=aptos-keyless-trusted-setup-contributions
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

        if [ $branch == "--local" ]; then
            echo
            echo "Using current repo in $repodir"
            keylessrepo=$repodir
        else
            keylessrepo=aptos-keyless-circuit-$(echo $branch | tr / -)
            echo
            echo "Cloning or updating aptos-keyless-circuit:$branch in $keylessrepo..."
            if [ -d $keylessrepo ]; then 
                echo "Looks like repo is cloned. Updating..."
                pushd $keylessrepo; git checkout $branch; popd
            else
                echo "Cloning..."
                git clone git@github.com:aptos-labs/aptos-keyless-circuit $keylessrepo
                pushd $keylessrepo; git checkout $branch; popd
            fi
        fi
        pushd $keylessrepo 
        {
            pushd templates/
            {
                echo
                echo "Re-compiling circuit. This will take several seconds..."
                circom -l . main.circom --r1cs --wasm --sym 

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
        popd
    }
    popd
}

if [ "$#" -ne 2 ]; then
    print_usage
    exit 1
fi

pushd $repodir/
    trusted_setup "$1" "$2"
popd
