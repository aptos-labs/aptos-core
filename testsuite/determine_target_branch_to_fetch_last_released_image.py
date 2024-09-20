import sys
import subprocess
import re

def get_latest_release_branch():
    """Get the latest aptos-release-vX.Y branch"""
    result = subprocess.run(['git', 'ls-remote', '--heads', 'origin', 'aptos-release-v*'], capture_output=True, text=True)
    branches = re.findall(r'refs/heads/(aptos-release-v\d+\.\d+)$', result.stdout, re.MULTILINE)
    
    if not branches:
        raise ValueError("No aptos-release branches found")
    
    # Return the highest version release branch
    return max(branches, key=lambda x: [int(n) for n in x.split('v')[1].split('.')])

def get_latest_branch_for_previous_major(major):
    """Get the latest aptos-release-v(previous_major).Y branch"""
    prev_major = int(major) - 1
    result = subprocess.run(['git', 'ls-remote', '--heads', 'origin', f'aptos-release-v{prev_major}.*'], capture_output=True, text=True)
    branches = re.findall(rf'refs/heads/(aptos-release-v{prev_major}\.\d+)$', result.stdout, re.MULTILINE)
    
    if branches:
        return max(branches, key=lambda x: int(x.split('.')[-1]))
    return None

def get_release_branch_from_tag(tag):
    """Extract release branch name from the aptos-node-vX.Y.Z-* tag"""
    match = re.match(r'^aptos-node-v(\d+)\.(\d+)\.\d+-.*$', tag)
    if match:
        major, minor = match.groups()
        return f'aptos-release-v{major}.{minor}'
    return None

def get_latest_tag_in_history(branch, max_commits=100):
    """Search commit history for a tag like aptos-node-vX.Y.Z-*"""
    result = subprocess.run(['git', 'log', '-n', str(max_commits), '--format=%H', branch], capture_output=True, text=True)
    commits = result.stdout.strip().split('\n')
    
    for commit in commits:
        tags = subprocess.run(['git', 'tag', '--points-at', commit], capture_output=True, text=True).stdout.strip().split('\n')
        
        # Find the first tag that matches aptos-node-vX.Y.Z-*
        for tag in tags:
            if re.match(r'^aptos-node-v\d+\.\d+\.\d+-.*$', tag):
                return tag
    return None

def branch_exists(branch):
    """Check if a branch exists in the remote repository"""
    result = subprocess.run(['git', 'ls-remote', '--heads', 'origin', branch], capture_output=True)
    return result.returncode == 0

def determine_target_branch(base_branch, max_commits=100):
    """Determine the appropriate target branch based on the base branch"""
    if base_branch == 'main':
        # For main, use the latest release branch
        return get_latest_release_branch()

    elif base_branch.startswith('aptos-release-v'):
        # If the base branch is a release branch, find the previous release branch
        match = re.match(r'^aptos-release-v(\d+)\.(\d+)', base_branch)
        if match:
            major, minor = match.groups()
            if minor == '0':
                return get_latest_branch_for_previous_major(major)
            else:
                return f'aptos-release-v{major}.{int(minor) - 1}'
        else:
            raise ValueError(f"Invalid release branch format: {base_branch}")

    else:
        # For non-main/non-release branches, search history for a tag like aptos-node-vX.Y.Z-*
        print(f"Searching {base_branch}'s history for the latest aptos-node tag.", file=sys.stderr)
        latest_tag = get_latest_tag_in_history(base_branch, max_commits)

        if latest_tag:
            release_branch = get_release_branch_from_tag(latest_tag)
            if release_branch and branch_exists(release_branch):
                return release_branch
            else:
                raise ValueError(f"Release branch {release_branch} does not exist")
        else:
            raise ValueError(f"No aptos-node tag found in the last {max_commits} commits of {base_branch}. Please rebase or correct the branch.")

if __name__ == '__main__':
    if len(sys.argv) != 2:
        print("Usage: python determine_target_branch.py <base_branch>", file=sys.stderr)
        sys.exit(1)

    base_branch = sys.argv[1]
    try:
        target_branch = determine_target_branch(base_branch)
        print(f"Target branch is {target_branch}", file=sys.stderr)
        print(target_branch)
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
