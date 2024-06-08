#!/bin/bash

# Takes a snapshot of the fuzzing session on the remote machine.
#
# Usage: ./scripts/take_snapshot.sh <host> [snapshot_name]
#
# Expects SSH host has been set up in the `~/.ssh/config` file.
#   i.e. the host is accessible via `ssh <host>`.
#
# Expects `aptos-core` directly under the home directory.
# Default snapshot name is 'vm-running'.
# Results are stored in `vm-results` directory.

MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)

results_dir=$MOVE_SMITH_DIR/vm-results
mkdir -p $results_dir

remote_move_smith="~/aptos-core/third_party/move/tools/move-smith"

function take_snapshot() {
    local host=$1
    local snapshot_name=${2:-"vm-running"}
    local tarball="$snapshot_name.tar.gz"

    echo "Taking snapshot from $host..."
    echo "Snapshot name: $snapshot_name"

    if ! grep -q "coverage" <<< $(ssh $host "cd $remote_move_smith && ls"); then
        printf "\033[41mRemote machine doesn't have coverage results generated, please check.\033[0m\n"
        sleep 1
        coverage_dir=""
    else
        coverage_dir="coverage/"
    fi

    local snapshot_dir=$results_dir/$snapshot_name
    mkdir -p $snapshot_dir

    cmd="cd $remote_move_smith && "
    cmd+='echo "Branch: $(git rev-parse --abbrev-ref HEAD), Commit: $(git log -1 --pretty=format:'\''%H - %s'\'')"'
    ssh $host "$cmd" 2>&1 | tee $snapshot_dir/git.log

    cmd="cd $remote_move_smith && "
    cmd+="tar -czf $tarball $coverage_dir fuzz/ logs/ src/ scripts/ Cargo.toml"
    echo "Running: $cmd"
    ssh $host "$cmd"

    if [ $? -ne 0 ]; then
        echo "Failed to create snapshot tarball, aborting"
        exit 1
    fi
    echo "Created snapshot $snapshot_name.tar.gz on remote machine"

    scp $host:$remote_move_smith/$tarball $results_dir
    if [ ! -f $results_dir/$tarball ]; then
        echo "Failed to copy snapshot, aborting"
        exit 1
    fi
    echo "Copied snapshot to $results_dir/$tarball"

    tar -xzvf $results_dir/$tarball -C $snapshot_dir
    echo "Snapshot extracted to $snapshot_dir"

    last_log=$(ls -rt $snapshot_dir/logs | tail -n 1)
    cp $snapshot_dir/logs/$last_log $snapshot_dir/fuzz.log

    for i in $(ls $snapshot_dir/logs); do
        mv $snapshot_dir/logs/$i $snapshot_dir/logs/old-$(basename $i).txt
    done
    echo "Copied newest log to $snapshot_dir/fuzz.log"

    if ! grep -q "coverage" <<< $(ls $snapshot_dir); then
        printf "\033[41mCoverage results were not found in the snapshot!!!\033[0m\n"
        echo "Please generate coverage results on the remote machine for most accurate results."
        exit 1
    fi
}

if [ "$#" -ne 2 ]; then
    echo "Usage: ./scripts/take_snapshot.sh <host> [snapshot_name]"
    echo "Make sure you can do 'ssh <host>'"
    echo "Make sure '~/aptos-core' exists on the remote machine"
    echo "Default snapshot name is 'vm-running'"
    exit 1
fi

take_snapshot $1 $2
