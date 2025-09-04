#!/bin/bash

# PAT_TOKEN and TARGET_BRANCH are passed in as environment variables.

TARGET_REPO="velor-chain/velor-indexer-processors"
WORKFLOW_NAME="lint.yaml"
MAX_RETRIES=30
RETRY_INTERVAL=60 # seconds

echo "Monitoring workflow $WORKFLOW_NAME for branch $TARGET_BRANCH in $TARGET_REPO"

for ((i=1; i<=MAX_RETRIES; i++)); do
    # Get the latest run of the workflow for the target branch
    response=$(curl -s -H "Authorization: token $PAT_TOKEN" \
          "https://api.github.com/repos/$TARGET_REPO/actions/workflows/$WORKFLOW_NAME/runs?branch=$TARGET_BRANCH&status=in_progress")

    # Check if there is an in-progress run
    if echo "$response" | grep -q '"in_progress"'; then
        echo "Workflow still in progress... waiting."
        sleep $RETRY_INTERVAL
    else
        # Check the latest completed run for the branch
        response=$(curl -s -H "Authorization: token $PAT_TOKEN" \
        "https://api.github.com/repos/$TARGET_REPO/actions/workflows/$WORKFLOW_NAME/runs?branch=$TARGET_BRANCH&status=completed")
        echo "$response"
        total_count=$(echo "$response" | jq -r '.total_count')
        if [ "$total_count" -gt 0 ]; then
            conclusion=$(echo "$response" | jq -r '.workflow_runs[0].conclusion')
        else
            echo "No workflow runs found for branch $TARGET_BRANCH"
            sleep $RETRY_INTERVAL
            continue
        fi
        # If the workflow succeeds, exit with a zero status
        if [ "$conclusion" == "success" ]; then
            echo "Workflow completed successfully!"
            exit 0
        fi
        # Otherwise, we retry.
    fi
done

echo "Workflow did not complete within the timeout period."
exit 1