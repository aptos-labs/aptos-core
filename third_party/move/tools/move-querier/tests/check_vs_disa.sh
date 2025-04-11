# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
# This script validates the functionality (works or not) of the Move querier, using the Move disassemlber as the baseline.

#!/bin/bash
process_file() {
    local file="$1"
    local aptos="$2"

    if [ -f "${file}.type" ]; then
        rm "${file}.type"
    fi

    if [ -f "${file}.asm" ]; then
        rm "${file}.asm"
    fi

    if [ -f "${file}.cg.dot" ]; then
        rm "${file}.cg.dot"
    fi

    local res_file=$( ${aptos} move query --check-bytecode-type --bytecode-path "$file" | jq -r '.Result')

    if [ -z "$res_file" ]; then
        echo "Error: bytecode type check failed on $file"
        exit 1
    fi

    read -r type < ${res_file}

    case "$type" in
        script)
            local dis_res=$( ${aptos} move disassemble --is-script --bytecode-path "$file" | jq -r '.Result')
            echo "step 4"
            echo $dis_res
            local query_res=$( ${aptos} move query --dump-call-graph --bytecode-path "$file" | jq -r '.Result')

            if [ -z "$dis_res" ] || [ -z "$query_res" ]; then
                echo "Test failed on $file"
                echo "Disassemble result: $dis_res"
                echo "Query result: $query_res"
                exit 1
            else
                echo "Test passed on script $file"
                rm "${file}.asm"
                rm "${file}.dot"
            fi
            ;;    
        module)
            local dis_res=$( ${aptos} move disassemble --bytecode-path "$file" | jq -r '.Result')
            local query_res=$( ${aptos} move query --dump-call-graph --bytecode-path "$file" | jq -r '.Result')
            
            if [ -z "$dis_res" ] || [ -z "$query_res" ]; then
                echo "Test failed on $file"
                echo "Disassemble result: $dis_res"
                echo "Query result: $query_res"
                exit 1
            else
                echo "Test passed on module $file"
                rm "${file}.asm"
                rm "${file}.cg.dot"
            fi
            ;;
        *)
            echo "Invalid bytecode type: $res_file"
            exit 1
            ;;
    esac

    if [ -f "${file}.type" ]; then
        rm "${file}.type"
    fi
}

MV_DIR="$1"
APTOS_TOOL="$2"
for mv_file in `find $MV_DIR -name "*.mv"`; 
do
     process_file "$mv_file" $APTOS_TOOL
done





