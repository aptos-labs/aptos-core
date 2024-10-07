#!/bin/bash

# Poll for Workflow Run and Wait for Job Completion

# Unique identifier for the run (commit_hash or UUID passed via event payload)
UUID="${GITHUB_SHA}"

# If a run_id is already known, use it directly to check the run status
if [ -f ".cached_run_id" ]; then
  run_id=$(cat .cached_run_id)
  echo "Using cached run_id: $run_id"
else
  echo "Polling for the workflow run with UUID: $UUID."

  attempts=0
  max_attempts=5  # Number of attempts to find the run_id
  sleep_interval=30  # Time to wait between attempts (in seconds)

  while [ $attempts -lt $max_attempts ]; do
    echo "Polling for the workflow run. Attempt $((attempts+1)) of $max_attempts..."

    # Get the workflow runs for the repository
    response=$(curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    "https://api.github.com/repos/aptos-labs/aptos-indexer-processors/actions/runs?event=repository_dispatch&branch=main")

    # Filter the workflow run by the unique run-name commit hash
    run_id=$(echo "$response" | jq -r ".workflow_runs[] | select(.name | test(\"$UUID\")) | .id")

    if [ -n "$run_id" ]; then
      echo "Found workflow run with ID: $run_id"
      echo "$run_id" > .cached_run_id  # Save the run_id to cache
      break
    else
      echo "No matching workflow run found yet. Retrying in $sleep_interval seconds..."
      attempts=$((attempts + 1))
      sleep $sleep_interval
    fi
  done
fi

# If we still don't have a run_id, exit the job
if [ -z "$run_id" ]; then
  echo "Workflow run not found after $max_attempts attempts. Exiting."
  exit 1
fi

# Now that we have the run_id (cached or newly found), proceed to poll job status
jobs_url="https://api.github.com/repos/aptos-labs/aptos-indexer-processors/actions/runs/${run_id}/jobs"

# Poll the job status until completion
job_completed=false
max_job_attempts=20  # Adjust based on how long you expect the job to run
job_attempts=0
sleep_interval=60  # Adjust polling interval as needed

while [ "$job_completed" == false ] && [ $job_attempts -lt $max_job_attempts ]; do
  echo "Polling for job status. Attempt $((job_attempts+1)) of $max_job_attempts..."
  jobs_response=$(curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" "$jobs_url")

  # Loop through the jobs and check their status
  for job in $(echo "$jobs_response" | jq -r '.jobs[] | @base64'); do
    _jq() {
      echo "${job}" | base64 --decode | jq -r "${1}"
    }

    job_name=$(_jq '.name')
    job_id=$(_jq '.id')
    job_status=$(_jq '.status')
    job_conclusion=$(_jq '.conclusion')

    echo "Checking job: $job_name (Job ID: $job_id)"
    echo "Job status: $job_status"
    echo "Job conclusion: $job_conclusion"

    # Check if the job has completed
    if [ "$job_status" == "completed" ]; then
      job_completed=true
      if [ "$job_conclusion" == "success" ]; then
        echo "Job completed successfully!"
        exit 0  # Exit with success
      else
        echo "Job failed!"
        exit 1  # Exit with failure
      fi
    fi
  done

  # Sleep before the next polling attempt
  echo "Job is still in progress. Waiting $sleep_interval seconds before polling again..."
  sleep $sleep_interval
  job_attempts=$((job_attempts + 1))
done

# If the job hasn't completed within the allowed attempts, exit with an error
if [ "$job_completed" == false ]; then
  echo "Job did not complete within the expected time. Exiting with failure."
  exit 1
fi
