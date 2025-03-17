# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# Bash scripts to test the move query tool with move packages
# Usage: ./check_vs_disa.sh <move_dir> <aptos_tool>
# - `move_dir`: The directory containing the Move packages to test (each package should keep all the bytecode files in a subdirectory named "bytecode").
# - `aptos_tool`: The path to the Aptos CLI tool (e.g., `./aptos`).

#!/bin/bash
process_pkg() {
    local pkg="$1"
    local aptos="$2"

    for asm_file in ${pkg}/*.asm; do
        if [ -f "$asm_file" ]; then
            rm "$asm_file"
        fi
    done

    for dot_file in ${pkg}/*.dot; do
        if [ -f "$dot_file" ]; then
            rm "$dot_file"
        fi
    done

    local dis_res=$( ${aptos} move disassemble --package-path "$pkg" | jq -r '.Result')
    local cg_res=$( ${aptos} move query --dump-call-graph --package-path "$pkg" | jq -r '.Result')
    local dep_res=$( ${aptos} move query --dump-dep-graph --package-path "$pkg" | jq -r '.Result')


    if [ -z "$dis_res" ] || [ -z "$cg_res" ] || [ -z "$dep_res" ]; then
        echo "Test failed on $pkg"
        echo "Disassemble result: $dis_res"
        echo "Call graph result: $cg_res"
        echo "Dependency graph result: $dep_res"
    else
        echo "Test passed on $pkg"
    fi
}

MV_DIR="$1"
APTOS_TOOL="$2"
for mv_pkg in `find $MV_DIR -name "bytecode"`; 
do
    process_pkg "$mv_pkg" $APTOS_TOOL
done





