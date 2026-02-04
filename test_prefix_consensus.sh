#!/bin/bash

# Script to test prefix consensus with divergent inputs

echo "=== Cleaning up old output files ==="
rm -f /tmp/prefix_consensus_output_*.json
echo "Deleted all /tmp/prefix_consensus_output_*.json files"
echo ""

echo "=== Running divergent inputs test ==="
cargo test -p smoke-test test_prefix_consensus_divergent_inputs -- --nocapture
TEST_EXIT_CODE=$?
echo ""

echo "=== Test exit code: $TEST_EXIT_CODE ==="
echo ""

echo "=== Output files created ==="
ls -lh /tmp/prefix_consensus_output_*.json 2>/dev/null || echo "No output files found"
echo ""

echo "=== Validator Inputs and Outputs ==="
VALIDATOR_NUM=1
for file in /tmp/prefix_consensus_output_*.json; do
    if [ -f "$file" ]; then
        echo ""
        echo "Validator $VALIDATOR_NUM:"
        echo "  Party ID: $(jq -r '.party_id' "$file")"
        echo "  Epoch: $(jq -r '.epoch' "$file")"
        echo ""
        echo "  Input vector (length $(jq '.input | length' "$file")):"
        jq -r '.input[] | "    [" + (. | .[0:16]) + "..." + (. | .[-8:]) + "]"' "$file"
        echo ""
        echo "  v_low (length $(jq '.v_low | length' "$file")):"
        jq -r '.v_low[] | "    [" + (. | .[0:16]) + "..." + (. | .[-8:]) + "]"' "$file"
        echo ""
        echo "  v_high (length $(jq '.v_high | length' "$file")):"
        jq -r '.v_high[] | "    [" + (. | .[0:16]) + "..." + (. | .[-8:]) + "]"' "$file"
        echo ""
        VALIDATOR_NUM=$((VALIDATOR_NUM + 1))
    fi
done

echo ""
echo "=== Summary ==="
FILE_COUNT=$(ls -1 /tmp/prefix_consensus_output_*.json 2>/dev/null | wc -l | tr -d ' ')
echo "Total output files: $FILE_COUNT"

if [ "$FILE_COUNT" -gt 0 ]; then
    echo ""
    echo "=== Comparing Outputs ==="
    echo "Checking if all validators have:"
    echo "  - Different inputs at position 2"
    echo "  - Same v_low (maximum common prefix)"
    echo "  - Same v_high"
fi

exit $TEST_EXIT_CODE
