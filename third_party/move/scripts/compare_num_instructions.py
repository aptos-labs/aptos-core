#! /usr/bin/env python3

# This script can be used to compare the number of bytecode instructions in two
# build folders of the *same* project.
#
# The only difference between the two folders should be how the project was built:
# e.g., with different versions of the compiler, with different optimizations, etc.
#
# Usage:
#   python3 compare_num_instructions.py --help

import argparse
import glob
import subprocess
import sys
import re
import os


def main():
    p = argparse.ArgumentParser(
        description="Compare two build folders of the *same* project for number of bytecode instructions"
    )
    p.add_argument("dir1", help="Directory containing the first build folder")
    p.add_argument("dir2", help="Directory containing the second build folder")
    args = p.parse_args()
    # Get the absolute path of the directories.
    dir1 = os.path.abspath(args.dir1)
    dir2 = os.path.abspath(args.dir2)
    # Check if the directories exist.
    if not os.path.isdir(dir1):
        print(f"Directory {dir1} does not exist")
        sys.exit(1)
    if not os.path.isdir(dir2):
        print(f"Directory {dir2} does not exist")
        sys.exit(1)
    # Save the original working directory and change to the script directory.
    original_working_dir = os.getcwd()
    script_dir = os.path.dirname(__file__)
    os.chdir(script_dir)
    # Build the move-disassembler in release mode.
    subprocess.run(["cargo", "build", "--release", "-p", "move-disassembler"])
    (total_1, total_2) = (0, 0)  # Total number of instructions.
    tally = []  # List of tuples containing the percentage increase and the file name.
    common_path = os.path.commonpath([dir1, dir2])
    # Compute for all bytecode files in the build folders.
    for file1 in glob.glob(dir1 + "/**/*.mv", recursive=True):
        file2 = file1.replace(dir1, dir2)
        if not os.path.exists(file2):
            print(f"There is no file comparable to {file1} in {dir2}")
            sys.exit(1)
        (i1, i2) = (count_instructions(file1), count_instructions(file2))
        percent_inc = percentage_change(i1, i2)
        tally.append((percent_inc, file1.removeprefix(common_path)))
        total_1 += i1
        total_2 += i2
    # Print the results per file, in order of higher increase earlier.
    for percent_inc, file in sorted(tally, reverse=True):
        print(f"{file}: {percent_inc:.1f}%")
    # Print the total percentage increase across all files.
    percent_inc = percentage_change(total_1, total_2)
    print(
        f"Total percentage change in instructions: {percent_inc:.1f}%, {total_1} -> {total_2}"
    )
    # Change back to the original working directory.
    os.chdir(original_working_dir)


def percentage_change(i1, i2):
    """Percentage change from `i1` to `i2`."""
    if i1 != 0:
        return ((i1 - i2) * 100.0) / i1
    return 0


# Regex pattern to match the total number of instructions in the disassembler output.
INSTRUCTION_COUNT_RE = re.compile(r"// Total number of instructions: (\d+)")


def count_instructions(file_name):
    """
    Count the number of bytecode instructions in the given file.
    This requires disassembling the move bytecode.
    """
    result = subprocess.run(
        [
            "../../../target/release/move-disassembler",
            "--skip-code",
            "--skip-locals",
            "--print-bytecode-stats",
            "--bytecode",
            file_name,
        ],
        stdout=subprocess.PIPE,
    )
    result = result.stdout.decode("utf-8")
    # Get the second last line, which contains the instruction count.
    result = result.splitlines()[-2]
    instructions = re.match(INSTRUCTION_COUNT_RE, result).group(1)
    return int(instructions)


if __name__ == "__main__":
    if sys.version_info < (3, 9):
        print("This script requires Python 3.9 or later")
        sys.exit(1)
    main()
