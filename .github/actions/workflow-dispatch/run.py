#!/usr/bin/env python3

# A wrapper around https://cli.github.com/manual/gh_workflow_run
# The arguments to this script should be the same as "gh workflow run"

import argparse
import subprocess
import os
import time

parser = argparse.ArgumentParser()
parser.add_argument(
    "workflow_file",
    help="The workflow file to run",
)
parser.add_argument(
    "--field",
    "-F",
    help="Add a string parameber in key=value format",
    action="append",
)
parser.add_argument(
    "--ref",
    "-r",
    help="The branch or tag name which contains the workflow file",
    required=True,
)

args = parser.parse_args()

workflow_file = args.workflow_file
workflow_ref = args.ref
input_fields = args.field
input_fields_args = [f"--field={field}" for field in input_fields]

my_env = os.environ.copy()
my_env["PAGER"] = ""  # disable the pager

# Trigger the workflow
subprocess.run(
    [
        "gh",
        "workflow",
        "run",
        workflow_file,
        f"--ref={workflow_ref}",
        *input_fields_args,
    ],
    env=my_env,
)

# sleep a bit to get the run ID
time.sleep(5)

# Get the run ID
out = subprocess.check_output(
    [
        "gh",
        "run",
        "list",
        "--workflow",
        workflow_file,
        "--branch",
        workflow_ref,
        "--limit",
        "10",
        "--json",
        "databaseId",
        "--jq",
        ".[0].databaseId",
    ],
    env=my_env,
)
run_id = out.decode().strip()

# Show the output of the run
print("=========================================")
subprocess.run(["gh", "run", "view", run_id], env=my_env)
print("=========================================")

# Tail the status
subprocess.run(["gh", "run", "watch", run_id, "--exit-status"], env=my_env)
