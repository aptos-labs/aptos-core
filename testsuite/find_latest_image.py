#!/usr/bin/env python3

# This script is used to find the latest built image tag. It has two modes where it can be used:
# 1. If the IMAGE_TAG environment variable is set, it will simply check that the image exists.
# 2. If the IMAGE_TAG environment variable is not set, it will find the latest image tag from git history,
#    assuming images are tagged with the git commit hash.

from forge import find_recent_images, image_exists
from forge_wrapper_core.shell import LocalShell
from forge_wrapper_core.git import Git

# gh output logic from determinator
from determinator import GithubOutput, write_github_output

import argparse
import os
import sys

# the image name in the repo to search for the image tag in
# all images are exported together, so this currently checks for validator-testing as well
IMAGE_NAME = "aptos/validator"
# the environment variable containing the image tag to check for existence
IMAGE_TAG_ENV = "IMAGE_TAG"
# if running in github actions, this is the output key that will contain the latest image tag
GH_OUTPUT_KEY = "IMAGE_TAG"


def main():
    shell = LocalShell()
    git = Git(shell)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--variant",
        "-v",
        help="A build variant",
        action="append",
        dest="variants",
        default=[],
    )
    args = parser.parse_args()

    # If the IMAGE_TAG environment variable is set, check that
    if IMAGE_TAG_ENV in os.environ and os.environ[IMAGE_TAG_ENV]:
        image_tag = os.environ[IMAGE_TAG_ENV]
        if not image_exists(shell, IMAGE_NAME, image_tag):
            sys.exit(1)

    variants = args.variants
    print(f"Finding latest image with build variants: {variants}")
    variant_prefixes = [f"{v}_" if v != "" else "" for v in variants]
    print(f"With prefixes: {variant_prefixes}")

    # Find the latest image from git history
    num_images_to_find = 1  # for the purposes of this script, this is always 1
    images = list(
        find_recent_images(shell, git, num_images_to_find, IMAGE_NAME, variant_prefixes)
    )
    print(f"Found latest images: {images}")

    # write the output to Github outputs (via stdout)
    git_sha_set = set([x.split("_")[-1] for x in images])  # trim to get the base sha
    assert len(git_sha_set) == num_images_to_find
    git_sha = git_sha_set.pop()
    print(f"Exporting latest image as base GIT_SHA: {git_sha}")
    latest_image_gh_output = GithubOutput(GH_OUTPUT_KEY, git_sha)
    try:
        write_github_output(latest_image_gh_output)
    except Exception as e:
        print(e)
        print(
            "This may be an indication that you're running locally or the environment is not configured correctly"
        )


if __name__ == "__main__":
    main()
