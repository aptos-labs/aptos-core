#!/usr/bin/env python3

import Levenshtein
import os
import re
import subprocess

# This file defines the branch commit checker script. The script will clone
# the private and public aptos-core repositories into the parent directory
# and verify the branch invariants defined in action.yaml.
#
# To run the script, simply install the requirements and invoke python, e.g.,:
#  pip3 install -r .github/actions/check-branch-commits/requirements.txt
#  python3 .github/actions/check-branch-commits/check_branch_commits.py
#
# Note: to display the commit messages during the run, set DISPLAY_COMMIT_MESSAGE
# to True (below).

# TODO: introduce a testing framework for this.

# Standard branch names
MAIN_BRANCH_NAME = "main"
DEVNET_BRANCH_NAME = "devnet"
TESTNET_BRANCH_NAME = "testnet"
MAINNET_BRANCH_NAME = "mainnet"

# Release branch constants
EXPECTED_NUM_PUBLIC_RELEASE_BRANCHES = 5
EXPECTED_NUM_PRIVATE_RELEASE_BRANCHES = 5
RELEASE_1_2_BRANCH_NAME = "aptos-release-v1.2"
RELEASE_1_3_BRANCH_NAME = "aptos-release-v1.3"
RELEASE_1_4_BRANCH_NAME = "aptos-release-v1.4"
RELEASE_BRANCH_TEMPLATE = "aptos-release-v"

# Relative file paths for public invariant exception files
PUBLIC_EXCEPTION_DIR = "public_exception_files/"
PUBLIC_DEVNET_NOT_PUBLIC_MAIN = PUBLIC_EXCEPTION_DIR + "public_devnet_not_public_main.txt"
PUBLIC_MAINNET_NOT_PUBLIC_MAIN = PUBLIC_EXCEPTION_DIR + "public_mainnet_not_public_main.txt"
PUBLIC_MAINNET_NOT_PUBLIC_TESTNET = PUBLIC_EXCEPTION_DIR + "public_mainnet_not_public_testnet.txt"
PUBLIC_TESTNET_NOT_PUBLIC_DEVNET = PUBLIC_EXCEPTION_DIR + "public_testnet_not_public_devnet.txt"
PUBLIC_TESTNET_NOT_PUBLIC_MAIN = PUBLIC_EXCEPTION_DIR + "public_testnet_not_public_main.txt"

# Relative file paths for private invariant exception files
PRIVATE_EXCEPTION_DIR = "private_exception_files/"
PRIVATE_MAIN_NOT_PUBLIC_MAIN = PRIVATE_EXCEPTION_DIR + "private_main_not_public_main.txt"
PUBLIC_MAIN_NOT_PRIVATE_MAIN = PRIVATE_EXCEPTION_DIR + "public_main_not_private_main.txt"

# Relative file paths for release branch invariant exception files
RELEASE_BRANCH_EXCEPTION_DIR = "release_branch_exception_files/"
RELEASE_1_2_PRIVATE_NOT_PUBLIC = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_2_private_not_public.txt"
RELEASE_1_2_PUBLIC_NOT_PRIVATE = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_2_public_not_private.txt"
RELEASE_1_3_PRIVATE_NOT_PUBLIC = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_3_private_not_public.txt"
RELEASE_1_3_PUBLIC_NOT_PRIVATE = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_3_public_not_private.txt"
RELEASE_1_4_PRIVATE_NOT_PUBLIC = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_4_private_not_public.txt"
RELEASE_1_4_PUBLIC_NOT_PRIVATE = RELEASE_BRANCH_EXCEPTION_DIR + "release_1_4_public_not_private.txt"

# Private repository constants
PRIVATE_APTOS_CORE_CLONE_TARGET_DIRECTORY = "private_aptos_core_clone"
PRIVATE_APTOS_CORE_COMMIT_URL_TEMPLATE = "https://github.com/aptos-labs/aptos-core-private/commit/{commit_hash}"
PRIVATE_APTOS_CORE_REPOSITORY_NAME = "aptos-core-private"
PRIVATE_APTOS_CORE_REPOSITORY_URL = "https://github.com/aptos-labs/aptos-core-private.git"

# Public repository constants
PUBLIC_APTOS_CORE_CLONE_TARGET_DIRECTORY = "public_aptos_core_clone"
PUBLIC_APTOS_CORE_COMMIT_URL_TEMPLATE = "https://github.com/aptos-labs/aptos-core/commit/{commit_hash}"
PUBLIC_APTOS_CORE_REPOSITORY_NAME = "aptos-core"
PUBLIC_APTOS_CORE_REPOSITORY_URL = "https://github.com/aptos-labs/aptos-core.git"

# Generic constants
EXPECTED_FILE_PATHS_TEMPLATE = ".github/actions/check-branch-commits/{file_name}"
MAX_NUM_COMMITS_TO_PRINT_PER_LIST = 20  # The maximum number of commits to print per commit list
PR_COMMIT_NUMBER_REGEX = r"\(#\d+\)"  # For example: (#7126), (#120), (#10000), etc.

# Security constants
DISPLAY_COMMIT_MESSAGE = False  # Only set to True for private/local runs, otherwise it may publicly leak commit messages!
HIDDEN_COMMIT_MESSAGE_STRING = "Commit message hidden for security reasons!"


# Returns a list of all the branch names that need to be tracked.
# New branches should be added here.
def get_all_branch_names():
    return [
        # Standard branch names
        MAIN_BRANCH_NAME,
        DEVNET_BRANCH_NAME,
        TESTNET_BRANCH_NAME,
        MAINNET_BRANCH_NAME,
        # Release branch names
        RELEASE_1_2_BRANCH_NAME,
        RELEASE_1_3_BRANCH_NAME,
        RELEASE_1_4_BRANCH_NAME,
    ]


# A class that represents a git commit and all of its metadata
class Commit:
    def __init__(self, hash, message):
        self.hash = hash
        self.message = message
        self.closest_matching_commit = None

    # Compares this commit against another commit and returns true iff the commits match
    def matches_commit(self, another_commit):
        return self.hash == another_commit.hash or self.message == another_commit.message

    # Compares this commit against another (potential cherry-pick) commit and returns true iff the messages match
    def matches_commit_with_cherry_pick(self, another_commit):
        # Strip out the PR numbers from the commit messages (to handle cherry-picks)
        commit_message = re.sub(PR_COMMIT_NUMBER_REGEX, "", self.message).strip()
        another_commit_message = re.sub(PR_COMMIT_NUMBER_REGEX, "", another_commit.message).strip()

        # Compare the stripped commit messages
        return commit_message == another_commit_message

    # Compares this commit message against a list of commit messages and stores the closest match
    def update_closest_matching_commit(self, another_commit_list):
        closest_matching_distance = float("inf")
        for another_commit in another_commit_list:
            distance = float(Levenshtein.distance(self.message, another_commit.message))
            if distance < closest_matching_distance:
                closest_matching_distance = distance
                self.closest_matching_commit = another_commit


# A class that represents a clone of the aptos-core repository
# (either the public or private repository).
class AptosCoreRepository:
    def __init__(self, private_repository):
        if private_repository:
            self.clone_target_directory = PRIVATE_APTOS_CORE_CLONE_TARGET_DIRECTORY
            self.commit_url_template = PRIVATE_APTOS_CORE_COMMIT_URL_TEMPLATE
            self.repository_name = PRIVATE_APTOS_CORE_REPOSITORY_NAME
            self.repository_url = PRIVATE_APTOS_CORE_REPOSITORY_URL
        else:
            self.clone_target_directory = PUBLIC_APTOS_CORE_CLONE_TARGET_DIRECTORY
            self.commit_url_template = PUBLIC_APTOS_CORE_COMMIT_URL_TEMPLATE
            self.repository_name = PUBLIC_APTOS_CORE_REPOSITORY_NAME
            self.repository_url = PUBLIC_APTOS_CORE_REPOSITORY_URL

        # Clone the repository before returning
        self.identified_release_branches = []
        self.clone_and_check_repository()

    # Clones the aptos-core repository to the clone directory
    # and checks out all the relevant branches.
    def clone_and_check_repository(self):
        # Get the current working directory
        working_directory = os.getcwd()

        # Create a clone directory and clone the repo at ../
        os.chdir("..")
        os.mkdir(self.clone_target_directory)
        os.chdir(self.clone_target_directory)
        subprocess.run(["git", "clone", self.repository_url])

        # Change directory into the cloned repository
        os.chdir(self.repository_name)

        # Fetch all the release branches
        release_branch_pattern = "*{release_branch_template}*".format(release_branch_template=RELEASE_BRANCH_TEMPLATE)
        (release_branches, _) = run_command_with_output_and_returncode(
            ["git", "branch", "--all", "--list", release_branch_pattern],
            "fetch_release_branches",
        )

        # Check out each release branch individually and save it to the set of release branches
        for release_branch in release_branches.decode().splitlines():  # Split the string on new lines
            release_branch = release_branch.strip()  # Remove leading and trailing whitespace
            _ = run_command_with_output_and_returncode(
                ["git", "checkout", "--track", release_branch],
                "track_release_branch",
            )
            self.identified_release_branches.append(release_branch)

        # Ensure the clone contains all history
        subprocess.run(["git", "fetch", "--all"])
        subprocess.run(["git", "pull", "--all"])

        # Checkout the various branches to update tracking and ensure they're available
        for branch_name in get_all_branch_names():
            subprocess.run(["git", "checkout", branch_name])
            subprocess.run(["git", "pull"])
            subprocess.run(["git", "log", "-1", "--pretty=oneline"])

        # Change back to the working directory
        os.chdir(working_directory)


# Runs the given command for the specified context and returns the
# output and return code.
def run_command_with_output_and_returncode(command, context):
    # Run the command
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output, errors = process.communicate()

    # Check for any errors
    if errors is not None and errors != b"":
        print("Found output on stderr for {context}: {errors}".format(context=context, errors=errors))

    # If the return code is non-zero, print out the output
    if process.returncode != 0:
        print(
            "WARNING! Found non-zero return code for {context}: {returncode}".format(
                context=context, returncode=process.returncode
            )
        )

    return (output, process.returncode)


# Prints the given message with a bulletpoint and indentation
def print_bullet_point(message):
    print("  - {message}".format(message=message))


# Prints the given message with an empty line before it
def print_with_empty_line(message):
    print("")  # Print a new line separator
    print(message)


# Prints out the commits in the given commit list (that were in the
# first repository, but not the second). If the list is too long,
# this function only prints out the maximum number of commits.
def print_commit_list(first_repo, second_repo, commit_list):
    num_commits_printed = 0
    for commit in commit_list:
        # Only print out the max number of commits
        if num_commits_printed >= MAX_NUM_COMMITS_TO_PRINT_PER_LIST:
            print("    - ... The rest of the commits have been omitted ...")
            return

        # Extract all commit information
        commit_hash = commit.hash
        commit_message = commit.message
        commit_url = first_repo.commit_url_template.format(commit_hash=commit_hash)

        # Extract all closest commit information
        closest_hash = commit.closest_matching_commit.hash
        closest_message = commit.closest_matching_commit.message
        closest_url = second_repo.commit_url_template.format(commit_hash=closest_hash)

        # Hide the commit message if we're not in private mode
        if not DISPLAY_COMMIT_MESSAGE:
            commit_message = HIDDEN_COMMIT_MESSAGE_STRING
            closest_message = HIDDEN_COMMIT_MESSAGE_STRING

        # Print the commit
        print(
            "    - Commit hash: {commit_hash}, message: {commit_message}, url: {commit_url}\n"
            "      Closest matching message: {closest_message}, closest commit: {closest_url}".format(
                commit_hash=commit_hash,
                commit_message=commit_message,
                commit_url=commit_url,
                closest_hash=closest_hash,
                closest_message=closest_message,
                closest_url=closest_url,
            )
        )
        num_commits_printed += 1


# Returns all the commits on the specified repo and branch
def get_commits_on_branch(repo, branch_name):
    # Construct the path to the aptos-core repository
    git_clone_path = "../{clone_directory_name}/{core_directory_name}".format(
        clone_directory_name=repo.clone_target_directory,
        core_directory_name=repo.repository_name,
    )

    # Check if the branch exists. If not, return an empty list.
    (_, returncode) = run_command_with_output_and_returncode(
        ["git", "-C", git_clone_path, "rev-parse", "--verify", branch_name],
        "get_commits_on_branch",
    )
    if returncode != 0:
        print(
            "WARNING! Branch {repo_name}:{branch_name} does not exist! Returning an empty list!".format(
                repo_name=repo.repository_name, branch_name=branch_name
            )
        )
        return []

    # Get all the commits from git log
    (commits, _) = run_command_with_output_and_returncode(
        ["git", "-C", git_clone_path, "log", "--pretty=oneline", branch_name, "--"],
        "get_commits_on_branch",
    )

    # For each commit, parse the hash and message
    commit_hashes_and_messages = []
    for commit in commits.decode().splitlines():  # Split the string on new lines
        commit_hash_and_message = commit.split(" ", 1)
        commit_hash = commit_hash_and_message[0]
        commit_message = commit_hash_and_message[1]
        commit_hashes_and_messages.append(Commit(commit_hash, commit_message))

    # Return the commits
    print_bullet_point(
        "Number of commits on {repo_name}:{branch_name}: {length}".format(
            repo_name=repo.repository_name,
            branch_name=branch_name,
            length=len(commit_hashes_and_messages),
        )
    )
    return commit_hashes_and_messages


# Fetches all the commit hashes to ignore from the given file
def get_commit_hashes_to_ignore(file_name):
    # Construct the full file path
    filepath = EXPECTED_FILE_PATHS_TEMPLATE.format(file_name=file_name)

    # Check if the file exists, otherwise print an error and return an empty list
    if not os.path.exists(filepath):
        print("WARNING! Could not find file: {filepath}".format(filepath=filepath))
        return []

    # Extract the commit hashes to ignore
    hashes_to_ignore = []
    with open(filepath) as file:
        for line in file:
            line = line.partition("#")[0]  # Ignore line comments
            commit_hash = line.rstrip()  # Ignore white space
            if commit_hash != "" and commit_hash is not None:
                hashes_to_ignore.append(commit_hash)
    return hashes_to_ignore


# Identifies all the commits in the first list but not in the second list
def get_commits_in_first_list_not_second(first_commit_list, second_commit_list):
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


# This function identifies all the commits on the first repository
# and branch that are not on the second repository and branch.
# The exception_file_name is the name of the file that contains
# the commit hashes that are expected to be missing.
def get_commits_on_first_branch_not_second(first_repo, first_branch_name, second_repo, second_branch_name, exception_file_name):
    print_with_empty_line(
        "Checking for commits on {first_repo_name}:{first_branch_name} but not on {second_repo_name}:{second_branch_name}...".format(
            first_repo_name=first_repo.repository_name,
            first_branch_name=first_branch_name,
            second_repo_name=second_repo.repository_name,
            second_branch_name=second_branch_name,
        )
    )

    # Get the commits
    first_branch_commits = get_commits_on_branch(first_repo, first_branch_name)
    second_branch_commits = get_commits_on_branch(second_repo, second_branch_name)

    # Identify the commits on the first branch, but not on the second
    missing_commits = get_commits_in_first_list_not_second(first_branch_commits, second_branch_commits)

    # Go through the missing commits and remove any that should be filtered out/ignored
    commit_hashes_to_ignore = get_commit_hashes_to_ignore(exception_file_name)
    print_bullet_point(
        "Found {length} exceptions in the exception file: {file_name}".format(
            length=len(commit_hashes_to_ignore), file_name=exception_file_name
        )
    )
    filtered_missing_commits = [commit for commit in missing_commits if commit.hash not in commit_hashes_to_ignore]
    num_filtered_commits = len(missing_commits) - len(filtered_missing_commits)
    print_bullet_point("Filtered out {number} missing commits using the exception file".format(number=num_filtered_commits))

    # Check if there are any exceptions in the exception file that we don't need
    num_unused_exceptions = len(commit_hashes_to_ignore) - num_filtered_commits
    if num_unused_exceptions > 0:
        print_bullet_point(
            "Some exceptions in the given exception file ({file_name}) aren't required! Didn't use: {number}".format(
                file_name=exception_file_name, number=num_unused_exceptions
            )
        )

    # Go through the missing commits and update the closest based on the message
    for missing_commit in filtered_missing_commits:
        missing_commit.update_closest_matching_commit(second_branch_commits)

    # Print out the commits on the first branch, but not on the second
    num_filtered_missing_commits = len(filtered_missing_commits)
    print_bullet_point(
        "Number of commits on {first_repo_name}:{first_branch_name} but not on {second_repo_name}:{second_branch_name}: {length}".format(
            first_repo_name=first_repo.repository_name,
            first_branch_name=first_branch_name,
            second_repo_name=second_repo.repository_name,
            second_branch_name=second_branch_name,
            length=num_filtered_missing_commits,
        )
    )
    if num_filtered_missing_commits > 0:
        print_bullet_point(
            "Commits on {first_repo_name}:{first_branch_name} but not on {second_repo_name}:{second_branch_name}:".format(
                first_repo_name=first_repo.repository_name,
                first_branch_name=first_branch_name,
                second_repo_name=second_repo.repository_name,
                second_branch_name=second_branch_name,
            )
        )
        print_commit_list(first_repo, second_repo, filtered_missing_commits)

    # Return the commit list
    return filtered_missing_commits


# Verifies the release branches in the public and private repositories
def verify_release_branches(public_aptos_core_repo, private_aptos_core_repo):
    print_with_empty_line("Verifying release branches...")

    # Print the release branches in the public repository
    print_bullet_point(
        "Identified release branches in {repo_name}: {branches}".format(
            repo_name=public_aptos_core_repo.repository_name,
            branches=public_aptos_core_repo.identified_release_branches,
        )
    )

    # Print the release branches in the private repository
    print_bullet_point(
        "Identified release branches in {repo_name}: {branches}".format(
            repo_name=private_aptos_core_repo.repository_name,
            branches=private_aptos_core_repo.identified_release_branches,
        )
    )

    # Verify the number of public release branches hasn't changed
    num_public_release_branches = len(public_aptos_core_repo.identified_release_branches)
    if num_public_release_branches != EXPECTED_NUM_PUBLIC_RELEASE_BRANCHES:
        print_bullet_point(
            "ERROR! The public release branches changed! Expected {expected} branches, but found: {actual}".format(
                expected=EXPECTED_NUM_PUBLIC_RELEASE_BRANCHES,
                actual=num_public_release_branches,
            )
        )

    # Verify the number of private release branches hasn't changed
    num_private_release_branches = len(private_aptos_core_repo.identified_release_branches)
    if num_private_release_branches != EXPECTED_NUM_PRIVATE_RELEASE_BRANCHES:
        print_bullet_point(
            "ERROR! The private release branches changed! Expected {expected} branches, but found: {actual}".format(
                expected=EXPECTED_NUM_PRIVATE_RELEASE_BRANCHES,
                actual=num_private_release_branches,
            )
        )


# Checks the branch invariants for the public repository and returns the
# number of invariants that failed as well as the total number of
# invariants checked.
def check_public_repository_invariants(public_aptos_core_repo):
    # Identify the commits on devnet/testnet/mainnet that are not on main
    devnet_commits_not_on_main = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        DEVNET_BRANCH_NAME,
        public_aptos_core_repo,
        MAIN_BRANCH_NAME,
        PUBLIC_DEVNET_NOT_PUBLIC_MAIN,
    )
    testnet_commits_not_on_main = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        TESTNET_BRANCH_NAME,
        public_aptos_core_repo,
        MAIN_BRANCH_NAME,
        PUBLIC_TESTNET_NOT_PUBLIC_MAIN,
    )
    mainnet_commits_not_on_main = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        MAINNET_BRANCH_NAME,
        public_aptos_core_repo,
        MAIN_BRANCH_NAME,
        PUBLIC_MAINNET_NOT_PUBLIC_MAIN,
    )

    # Identify the commits on testnet that are not on devnet
    testnet_commits_not_on_devnet = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        TESTNET_BRANCH_NAME,
        public_aptos_core_repo,
        DEVNET_BRANCH_NAME,
        PUBLIC_TESTNET_NOT_PUBLIC_DEVNET,
    )

    # Identify the commits on mainnet that are not on testnet
    mainnet_commits_not_on_testnet = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        MAINNET_BRANCH_NAME,
        public_aptos_core_repo,
        TESTNET_BRANCH_NAME,
        PUBLIC_MAINNET_NOT_PUBLIC_TESTNET,
    )

    # Calculate the number of failed invariants
    num_checked_invariants = 5
    num_failed_invariants = 0
    print_with_empty_line("Checked all invariants for the public repository!")
    if len(devnet_commits_not_on_main) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public devnet that were not on main!".format(
                num=len(devnet_commits_not_on_main),
            )
        )
        num_failed_invariants += 1
    if len(testnet_commits_not_on_main) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public testnet that were not on main!".format(
                num=len(testnet_commits_not_on_main),
            )
        )
        num_failed_invariants += 1
    if len(mainnet_commits_not_on_main) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public mainnet that were not on main!".format(
                num=len(mainnet_commits_not_on_main),
            )
        )
        num_failed_invariants += 1
    if len(testnet_commits_not_on_devnet) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public testnet that were not on devnet!".format(
                num=len(testnet_commits_not_on_devnet),
            )
        )
        num_failed_invariants += 1
    if len(mainnet_commits_not_on_testnet) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public mainnet that were not on testnet!".format(
                num=len(mainnet_commits_not_on_testnet),
            )
        )
        num_failed_invariants += 1

    return (num_failed_invariants, num_checked_invariants)


# Checks the branch invariants for the private repository and returns the
# number of invariants that failed as well as the total number of
# invariants checked.
def check_private_repository_invariants(public_aptos_core_repo, private_aptos_core_repo):
    # Compare the commits on the main branch across the public and private repositories
    main_commits_on_private_not_public = get_commits_on_first_branch_not_second(
        private_aptos_core_repo,
        MAIN_BRANCH_NAME,
        public_aptos_core_repo,
        MAIN_BRANCH_NAME,
        PRIVATE_MAIN_NOT_PUBLIC_MAIN,
    )
    main_commits_on_public_not_private = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        MAIN_BRANCH_NAME,
        private_aptos_core_repo,
        MAIN_BRANCH_NAME,
        PUBLIC_MAIN_NOT_PRIVATE_MAIN,
    )

    # Calculate the number of failed invariants
    num_checked_invariants = 2
    num_failed_invariants = 0
    print_with_empty_line("Checked all invariants for the private repository!")
    if len(main_commits_on_private_not_public) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on private main that were not on public main!".format(
                num=len(main_commits_on_private_not_public),
            )
        )
        num_failed_invariants += 1
    if len(main_commits_on_public_not_private) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public main branch that were not on private main!".format(
                num=len(main_commits_on_public_not_private),
            )
        )
        num_failed_invariants += 1

    return (num_failed_invariants, num_checked_invariants)


# Checks the release branch invariants and returns the number of
# invariants that failed as well as the total number of checks performed.
def check_release_branch_invariants(public_aptos_core_repo, private_aptos_core_repo):
    # Compare the commits on the release v1.2 branch across the public and private repositories
    release_1_2_commits_on_private_not_public = get_commits_on_first_branch_not_second(
        private_aptos_core_repo,
        RELEASE_1_2_BRANCH_NAME,
        public_aptos_core_repo,
        RELEASE_1_2_BRANCH_NAME,
        RELEASE_1_2_PRIVATE_NOT_PUBLIC,
    )
    release_1_2_commits_on_public_not_private = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        RELEASE_1_2_BRANCH_NAME,
        private_aptos_core_repo,
        RELEASE_1_2_BRANCH_NAME,
        RELEASE_1_2_PUBLIC_NOT_PRIVATE,
    )

    # Compare the commits on the release v1.3 branch across the public and private repositories
    release_1_3_commits_on_private_not_public = get_commits_on_first_branch_not_second(
        private_aptos_core_repo,
        RELEASE_1_3_BRANCH_NAME,
        public_aptos_core_repo,
        RELEASE_1_3_BRANCH_NAME,
        RELEASE_1_3_PRIVATE_NOT_PUBLIC,
    )
    release_1_3_commits_on_public_not_private = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        RELEASE_1_3_BRANCH_NAME,
        private_aptos_core_repo,
        RELEASE_1_3_BRANCH_NAME,
        RELEASE_1_3_PUBLIC_NOT_PRIVATE,
    )

    # Compare the commits on the release v1.4 branch across the public and private repositories
    release_1_4_commits_on_private_not_public = get_commits_on_first_branch_not_second(
        private_aptos_core_repo,
        RELEASE_1_4_BRANCH_NAME,
        public_aptos_core_repo,
        RELEASE_1_4_BRANCH_NAME,
        RELEASE_1_4_PRIVATE_NOT_PUBLIC,
    )
    release_1_4_commits_on_public_not_private = get_commits_on_first_branch_not_second(
        public_aptos_core_repo,
        RELEASE_1_4_BRANCH_NAME,
        private_aptos_core_repo,
        RELEASE_1_4_BRANCH_NAME,
        RELEASE_1_4_PUBLIC_NOT_PRIVATE,
    )

    # Calculate the number of failed invariants
    num_checked_invariants = 6
    num_failed_invariants = 0
    print_with_empty_line("Checked all invariants for the release branches!")
    if len(release_1_2_commits_on_private_not_public) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on private release v1.2 that were not on public release v1.2!".format(
                num=len(release_1_2_commits_on_private_not_public),
            )
        )
        num_failed_invariants += 1
    if len(release_1_2_commits_on_public_not_private) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public release v1.2 that were not on private release v1.2!".format(
                num=len(release_1_2_commits_on_public_not_private),
            )
        )
        num_failed_invariants += 1
    if len(release_1_3_commits_on_private_not_public) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on private release v1.3 that were not on public release v1.3!".format(
                num=len(release_1_3_commits_on_private_not_public),
            )
        )
        num_failed_invariants += 1
    if len(release_1_3_commits_on_public_not_private) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public release v1.3 that were not on private release v1.3!".format(
                num=len(release_1_3_commits_on_public_not_private),
            )
        )
        num_failed_invariants += 1
    if len(release_1_4_commits_on_private_not_public) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on private release v1.4 that were not on public release v1.4!".format(
                num=len(release_1_4_commits_on_private_not_public),
            )
        )
        num_failed_invariants += 1
    if len(release_1_4_commits_on_public_not_private) > 0:
        print_bullet_point(
            "ERROR! There were {num} commits found on public release v1.4 that were not on private release v1.4!".format(
                num=len(release_1_4_commits_on_public_not_private),
            )
        )
        num_failed_invariants += 1

    return (num_failed_invariants, num_checked_invariants)


# The main function ensures that the public and private aptos-core
# repository branches are valid and up-to-date with one another.
def main():
    # Clone the public aptos-core repository
    print_with_empty_line("Cloning the public aptos-core repository...")
    public_aptos_core_repo = AptosCoreRepository(False)

    # Clone the private aptos-core repository
    print_with_empty_line("Cloning the private aptos-core repository...")
    private_aptos_core_repo = AptosCoreRepository(True)

    # Print out the release branches that have been identified
    verify_release_branches(public_aptos_core_repo, private_aptos_core_repo)

    # Check the public repository for invariant violations
    print_with_empty_line("Checking the branch invariants for the public repository...")
    (
        num_public_check_failures,
        total_public_invariants_checked,
    ) = check_public_repository_invariants(public_aptos_core_repo)

    # Check the private repository for invariant violations
    print_with_empty_line("Checking the branch invariants for the private repository...")
    (
        num_private_check_failures,
        total_private_invariants_checked,
    ) = check_private_repository_invariants(public_aptos_core_repo, private_aptos_core_repo)

    # Check the public and private repositories for release branch invariant violations
    print_with_empty_line("Checking the release branch invariants for the public and private repositories...")
    (
        num_release_branch_check_failures,
        total_release_branch_invariants_checked,
    ) = check_release_branch_invariants(public_aptos_core_repo, private_aptos_core_repo)

    # Display the aggregate results
    print_with_empty_line("RESULT SUMMARY:")
    if num_public_check_failures == 0 and num_private_check_failures == 0:
        print_bullet_point("SUCCESS! All checks passed! There were no invariant failures.")
    else:
        if num_public_check_failures > 0:
            print_bullet_point(
                "FAILURE! {num_failures} invariant checks failed in the public repository (performed {total_checks} total checks). "
                "Scroll up to see the exact failures.".format(
                    num_failures=num_public_check_failures,
                    total_checks=total_public_invariants_checked,
                )
            )
        if num_private_check_failures > 0:
            print_bullet_point(
                "FAILURE! {num_failures} invariant checks failed in the private repository (performed {total_checks} total checks). "
                "Scroll up to see the exact failures.".format(
                    num_failures=num_private_check_failures,
                    total_checks=total_private_invariants_checked,
                )
            )
        if num_release_branch_check_failures > 0:
            print_bullet_point(
                "FAILURE! {num_failures} invariant checks failed in the release branches (performed {total_checks} total checks). "
                "Scroll up to see the exact failures.".format(
                    num_failures=num_release_branch_check_failures,
                    total_checks=total_release_branch_invariants_checked,
                )
            )
    print_bullet_point(
        "FAILED: {total_num_failures} out of {total_num_checks} checks failed!".format(
            total_num_failures=num_public_check_failures + num_private_check_failures + num_release_branch_check_failures,
            total_num_checks=total_public_invariants_checked
            + total_private_invariants_checked
            + total_release_branch_invariants_checked,
        )
    )


if __name__ == "__main__":
    main()
