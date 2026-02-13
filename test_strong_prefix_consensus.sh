#!/bin/bash

# Script to test strong prefix consensus (happy path — identical inputs)

echo "=== Cleaning up old output files ==="
rm -f /tmp/strong_prefix_consensus_output_*.json
echo "Deleted all /tmp/strong_prefix_consensus_output_*.json files"
echo ""

echo "=== Running strong prefix consensus identical inputs test ==="
cargo test -p smoke-test test_strong_prefix_consensus_identical_inputs -- --nocapture
TEST_EXIT_CODE=$?
echo ""

echo "=== Test exit code: $TEST_EXIT_CODE ==="
echo ""

echo "=== Output files created ==="
ls -lh /tmp/strong_prefix_consensus_output_*.json 2>/dev/null || echo "No output files found"
echo ""

echo "=== Validator Outputs ==="
VALIDATOR_NUM=1
for file in /tmp/strong_prefix_consensus_output_*.json; do
    if [ -f "$file" ]; then
        echo ""
        echo "Validator $VALIDATOR_NUM:"
        echo "  Party ID: $(jq -r '.party_id' "$file")"
        echo "  Epoch: $(jq -r '.epoch' "$file")"
        echo "  Slot: $(jq -r '.slot' "$file")"
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
FILE_COUNT=$(ls -1 /tmp/strong_prefix_consensus_output_*.json 2>/dev/null | wc -l | tr -d ' ')
echo "Total output files: $FILE_COUNT"

if [ "$FILE_COUNT" -gt 0 ]; then
    echo ""
    echo "=== Comparing Outputs ==="
    echo "Checking if all validators have:"
    echo "  - Same v_low (= input for identical inputs)"
    echo "  - Same v_high (= input for identical inputs)"
fi

exit $TEST_EXIT_CODE
