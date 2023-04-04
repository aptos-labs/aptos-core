#!/usr/bin/env python3

import Levenshtein
import os
import re
import subprocess
import sys

# This script runs the branch commit checker from the root of aptos-core.

# TODO: introduce a testing framework for this.

# Branch name constants
DEVNET_BRANCH_NAME = "devnet"
MAIN_BRANCH_NAME = "main"
MAINNET_BRANCH_NAME = "mainnet"
TESTNET_BRANCH_NAME = "testnet"

# File name constants
EXPECTED_ON_DEVNET_NOT_MAIN = "expected_on_devnet_not_main.txt"
EXPECTED_ON_MAINNET_NOT_MAIN = "expected_on_mainnet_not_main.txt"
EXPECTED_ON_MAINNET_NOT_TESTNET = "expected_on_mainnet_not_testnet.txt"
EXPECTED_ON_TESTNET_NOT_DEVNET = "expected_on_testnet_not_devnet.txt"
EXPECTED_ON_TESTNET_NOT_MAIN = "expected_on_testnet_not_main.txt"

# Generic constants
APTOS_CORE_DIRECTORY_NAME = "aptos-core"
APTOS_CORE_REPOSITORY_URL = "https://github.com/aptos-labs/aptos-core.git"
COMMIT_URL_TEMPLATE = "https://github.com/aptos-labs/aptos-core/commit/{commit_hash}"
EXPECTED_FILE_PATHS_TEMPLATE = ".github/actions/check-branch-commits/{file_name}"
GIT_CLONE_DIRECTORY_NAME = "aptos_core_clone"
PR_COMMIT_NUMBER_REGEX = r"\(#\d+\)" # For example: (#7126), (#120), (#10000), etc.

# A simple wrapper to hold all information related to a commit
class Commit:
  def __init__(self, hash, message):
    self.hash = hash
    self.message = message
    self.closest_matching_commit = None


  def matches_commit(self, another_commit):
    """Compares this commit against another commit and returns true iff the commits match"""
    return self.hash == another_commit.hash or self.message == another_commit.message

  def matches_commit_with_cherry_pick(self, another_commit):
    """Compares this commit against another (potential cherry-pick) commit and returns true iff the messages match"""
    # Strip out the PR numbers from the commit messages (to handle cherry-picks)
    commit_message = re.sub(PR_COMMIT_NUMBER_REGEX, '', self.message).strip()
    another_commit_message = re.sub(PR_COMMIT_NUMBER_REGEX, '', another_commit.message).strip()

    # Compare the stripped commit messages
    return commit_message == another_commit_message

  def update_closest_matching_commit(self, another_commit_list):
    """Compares this commit message against a list of commit messages and stores the closest match"""
    closest_matching_distance = float('inf')
    for another_commit in another_commit_list:
      distance = float(Levenshtein.distance(self.message, another_commit.message))
      if distance < closest_matching_distance:
        closest_matching_distance = distance
        self.closest_matching_commit = another_commit


# Note: we clone aptos-core instead of analyzing the aptos-core repo checked out by the
# github action because it interferes with the github action/script.
def clone_aptos_core_to_clone_directory():
  """Clones aptos-core to the clone directory with all branches and git history"""
  # Get the current working directory
  working_directory = os.getcwd()

  # Create a clone directory and clone the repo at ../
  os.chdir("..")
  os.mkdir(GIT_CLONE_DIRECTORY_NAME)
  os.chdir(GIT_CLONE_DIRECTORY_NAME)
  subprocess.run(["git", "clone", APTOS_CORE_REPOSITORY_URL])

  # Ensure the clone contains all history and branches
  os.chdir(APTOS_CORE_DIRECTORY_NAME)
  subprocess.run(["git", "fetch", "--all"])
  subprocess.run(["git", "pull", "--all"])

  # Checkout the various branches to update tracking and ensure they're available
  for branch_name in [DEVNET_BRANCH_NAME, MAIN_BRANCH_NAME, MAINNET_BRANCH_NAME, TESTNET_BRANCH_NAME]:
    subprocess.run(["git", "checkout", branch_name])
    subprocess.run(["git", "pull"])
    subprocess.run(["git", "log", "-1", "--pretty=oneline"])

  # Change back to the working directory
  os.chdir(working_directory)


def print_commit_list(commit_list):
  """Prints out the commits in the given commit list"""
  for commit in commit_list:
    # Extract all commit information
    commit_hash = commit.hash
    commit_message = commit.message
    commit_url = COMMIT_URL_TEMPLATE.format(commit_hash=commit_hash)

    # Extract all closest commit information
    closest_hash = commit.closest_matching_commit.hash
    closest_message = commit.closest_matching_commit.message
    closest_url = COMMIT_URL_TEMPLATE.format(commit_hash=closest_hash)

    # Print the commit
    print("  - Commit hash: {commit_hash}, message: {commit_message}, url: {commit_url}\n"
          "    Closest matching message: {closest_message}, closest commit: {closest_url}".format(commit_hash=commit_hash, commit_message=commit_message, commit_url=commit_url, closest_hash=closest_hash, closest_message=closest_message, closest_url=closest_url))


def get_commits_on_branch(branch_name):
  """Gets all the commits on the specified branch name"""
  # Construct the path to aptos-core repository
  git_clone_path = "../{clone_directory_name}/{core_directory_name}".format(clone_directory_name=GIT_CLONE_DIRECTORY_NAME, core_directory_name=APTOS_CORE_DIRECTORY_NAME)

  # Get all the commits from git log
  process = subprocess.Popen(["git", "-C", git_clone_path, "log", "--pretty=oneline", branch_name, "--"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
  commits, errors = process.communicate()
  if errors is not None and errors != b'':
    print("Found output on stderr for get_commits_on_branch: {errors}".format(errors=errors))

  # Split the commits into a list
  commits = commits.decode().splitlines() # Split the string on new lines

  # For each commit, parse the hash and message
  commit_hashes_and_messages = []
  for commit in commits:
    commit_hash_and_message = commit.split(" ", 1)
    commit_hash = commit_hash_and_message[0]
    commit_message = commit_hash_and_message[1]
    commit_hashes_and_messages.append(Commit(commit_hash, commit_message))

  # Return the commits
  print("Number of commits on {branch_name}: {length}".format(branch_name=branch_name, length=len(commit_hashes_and_messages)))
  return commit_hashes_and_messages


def get_commit_hashes_to_ignore(file_name):
  """Fetches all the commit hashes to ignore from the given file"""
  # Construct the full file path
  filepath = EXPECTED_FILE_PATHS_TEMPLATE.format(file_name=file_name)

  # Extract the commit hashes to ignore
  hashes_to_ignore = []
  with open(filepath) as file:
    for line in file:
      line = line.partition('#')[0] # Ignore line comments
      commit_hash = line.rstrip() # Ignore white space
      if commit_hash != "" and commit_hash is not None:
        hashes_to_ignore.append(commit_hash)
  return hashes_to_ignore


def get_commits_in_first_list_not_second(first_commit_list, second_commit_list):
  """Identifies all the commits in the first list but not in the second list"""
  commits_in_first_list_not_second = []
  for commit_in_first_list in first_commit_list:
    found = False
    for commit_in_second_list in second_commit_list:
      if commit_in_first_list.matches_commit(commit_in_second_list):
        found = True
        break

    # If the commit wasn't found, check for cherry-pick matches.
    # We don't do this automatically because it may be expensive to compute.
    if not found:
      for commit_in_second_list in second_commit_list:
        if commit_in_first_list.matches_commit_with_cherry_pick(commit_in_second_list):
          found = True
          break

    # If the commit still wasn't found, add it to the list
    if not found:
      commits_in_first_list_not_second.append(commit_in_first_list)
  return commits_in_first_list_not_second


def get_commits_on_first_branch_not_second(first_branch_name, second_branch_name, hashes_to_ignore_file_name):
  """Identifies all the commits on the first branch but not on the second branch"""
  print("") # Print a new line separator
  print("Checking for commits on {first_branch_name} but not on {second_branch_name}...".format(first_branch_name=first_branch_name, second_branch_name=second_branch_name))

  # Get the commits
  first_branch_commits = get_commits_on_branch(first_branch_name)
  second_branch_commits = get_commits_on_branch(second_branch_name)

  # Identify the commits on the first branch, but not on the second
  missing_commits = get_commits_in_first_list_not_second(first_branch_commits, second_branch_commits)

  # Go through the missing commits and remove any that should be filtered out/ignored
  commit_hashes_to_ignore = get_commit_hashes_to_ignore(hashes_to_ignore_file_name)
  print("Found {length} exceptions in the exception file: {file_name}".format(length=len(commit_hashes_to_ignore), file_name=hashes_to_ignore_file_name))
  filtered_missing_commits = [commit for commit in missing_commits if commit.hash not in commit_hashes_to_ignore]
  num_filtered_commits = len(missing_commits) - len(filtered_missing_commits)
  print("Filtered out {number} missing commits using the exception file".format(number=num_filtered_commits))

  # Check if there are any exceptions in the exception file that we don't need
  num_unused_exceptions = len(commit_hashes_to_ignore) - num_filtered_commits
  if num_unused_exceptions > 0:
    print("Some exceptions in the given exception file ({file_name}) aren't required! Didn't use: {number}".format(file_name=hashes_to_ignore_file_name, number=num_unused_exceptions))

  # Go through the missing commits and update the closest based on the message
  for missing_commit in filtered_missing_commits:
    missing_commit.update_closest_matching_commit(second_branch_commits)

  # Print out the commits on the first branch, but not on the second
  num_filtered_missing_commits = len(filtered_missing_commits)
  print("Number of commits on {first_branch_name} but not on {second_branch_name}: {length}".format(first_branch_name=first_branch_name, second_branch_name=second_branch_name, length=num_filtered_missing_commits))
  if num_filtered_missing_commits > 0:
    print("Commits on {first_branch_name} but not on {second_branch_name}:".format(first_branch_name=first_branch_name, second_branch_name=second_branch_name))
    print_commit_list(filtered_missing_commits)

  # Return the commit list
  return filtered_missing_commits


def main():
  # Create another aptos-core clone so we can fetch all branches and history
  clone_aptos_core_to_clone_directory()

  # Identify the commits on devnet/testnet/mainnet, but not on main
  devnet_commits_not_on_main = get_commits_on_first_branch_not_second(DEVNET_BRANCH_NAME, MAIN_BRANCH_NAME, EXPECTED_ON_DEVNET_NOT_MAIN)
  testnet_commits_not_on_main = get_commits_on_first_branch_not_second(TESTNET_BRANCH_NAME, MAIN_BRANCH_NAME, EXPECTED_ON_TESTNET_NOT_MAIN)
  mainnet_commits_not_on_main = get_commits_on_first_branch_not_second(MAINNET_BRANCH_NAME, MAIN_BRANCH_NAME, EXPECTED_ON_MAINNET_NOT_MAIN)

  # Identify the commits on testnet but not on devnet
  testnet_commits_not_on_devnet = get_commits_on_first_branch_not_second(TESTNET_BRANCH_NAME, DEVNET_BRANCH_NAME, EXPECTED_ON_TESTNET_NOT_DEVNET)

  # Identify the commits on mainnet but not on testnet
  mainnet_commits_not_on_testnet = get_commits_on_first_branch_not_second(MAINNET_BRANCH_NAME, TESTNET_BRANCH_NAME, EXPECTED_ON_MAINNET_NOT_TESTNET)

  # Return an error if there were any missing commits
  missing_commits = False
  if len(devnet_commits_not_on_main) > 0:
    print("There were commits found on devnet that were not on main!")
    missing_commits = True
  if len(testnet_commits_not_on_main) > 0:
    print("There were commits found on testnet that were not on main!")
    missing_commits = True
  if len(mainnet_commits_not_on_main) > 0:
    print("There were commits found on mainnet that were not on main!")
    missing_commits = True
  if len(testnet_commits_not_on_devnet) > 0:
    print("There were commits found on testnet that were not on devnet!")
    missing_commits = True
  if len(mainnet_commits_not_on_testnet) > 0:
    print("There were commits found on mainnet that were not on testnet!")
    missing_commits = True

  if missing_commits:
    print("Missing commits were found!")
    sys.exit(1)
  else:
    print("No missing commits were found!")


if __name__ == "__main__":
  main()
