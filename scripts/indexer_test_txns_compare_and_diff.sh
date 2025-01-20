#!/bin/bash

# Function to compare and handle diff logic
compare_and_diff() {
  local generated_file=$1
  local original_file=$2

  echo "Modified file path: $generated_file"
  echo "Original file path: $original_file"

  if [ -f "$original_file" ]; then
    echo "Original file exists, comparing with modified file."
    # Run diff and capture the output
    diff_output=$(diff -u "$original_file" "$generated_file" || true)

    if [ -n "$diff_output" ]; then
      echo "Differences found in $generated_file"
      diff_found=true
      modified_files="${modified_files}${generated_file}\n"  # Append the full path of the modified file
      echo "Diff output:"
      echo "$diff_output"
    else
      echo "No differences found for $generated_file."
    fi
  else
    echo "New file detected: $generated_file (no corresponding original file found)"
    new_file_found=true
    new_files="${new_files}${generated_file}\n"  # Append the full path of the new file

    # Treat as new file, but still run a diff (compare with /dev/null)
    diff_output=$(diff -u /dev/null "$generated_file" || true)
    if [ -n "$diff_output" ]; then
      echo "New file with diff found in $generated_file"
      echo "Diff output for new file:"
      echo "$diff_output"
    fi
  fi
}


# Initialize the flags
diff_found=false
new_file_found=false
new_files=""
modified_files=""

cd ecosystem/indexer-grpc/indexer-test-transactions/src || exit 1

echo "Starting comparison between new and original JSON files."

# C heck if the new_json_transactions folder exists
if [ ! -d "new_json_transactions" ]; then
  echo "Directory new_json_transactions does not exist. Exiting."
  exit 1
fi

# Loop over all subdirectories inside new_json_transactions
for folder in new_json_transactions/*; do
  if [ -d "$folder" ]; then  # Ensure it's a directory
    echo "Processing folder: $folder"

    # Check if the folder is for imported transactions
    if [[ "$folder" == *"imported_"* ]]; then
      # For imported transactions, process all files without any 'modified_' check
      for file in "$folder"/*.json; do
        if [ -f "$file" ]; then
          echo "Processing imported file: $file"
          base_file=$(basename "$file" .json)
          original_file="../indexer-test-transactions/src/json_transactions/$(basename $folder)/${base_file}.json"
          compare_and_diff "$file" "$original_file"
        fi
      done
    else
      # For scripted transactions, only process files that are prefixed with 'cleaned_'
      for file in "$folder"/cleaned_*.json; do
        if [ -f "$file" ]; then
          echo "Processing scripted file: $file"
          base_file=$(basename "$file" .json)
          original_file="../indexer-test-transactions/src/json_transactions/$(basename $folder)/${base_file}.json"
          compare_and_diff "$file" "$original_file"
        fi
      done
    fi
  else
    echo "Folder $folder is not a valid directory."
  fi
done

# Print all new files if found
if [ "$new_file_found" = "true" ] && [ -n "$new_files" ]; then
  echo "New files detected:"
  echo -e "$new_files"
else
  echo "No new files detected."
fi

# Print all modified files if found
if [ "$diff_found" = "true" ] && [ -n "$modified_files" ]; then
  echo "Modified files detected:"
  echo -e "$modified_files"
else
  echo "No modified files detected."
fi

# Debugging logs before setting outputs
echo "diff_found=$diff_found"
echo "new_file_found=$new_file_found"
echo "new_files=$new_files"
echo "modified_files=$modified_files"

# Set output flags
echo "diff_found=$diff_found" >> $GITHUB_OUTPUT
echo "new_file_found=$new_file_found" >> $GITHUB_OUTPUT
echo "new_files=$new_files" >> $GITHUB_OUTPUT  # Store new files as output
echo "modified_files=$modified_files" >> $GITHUB_OUTPUT  # Store modified files as output
echo "Comparison completed."
