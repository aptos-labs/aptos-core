#!/usr/bin/env python3

import argparse
import os
import sys

# This script is used to find the latest built image tag. It has two modes where it can be used:
# 1. If the IMAGE_TAG environment variable is set, it will simply check that the image exists.
# 2. If the IMAGE_TAG environment variable is not set, it will find the latest image tag from git history,
#    assuming images are tagged with the git commit hash.

from test_framework.logging import init_logging, log
from forge import find_recent_images, image_exists
from test_framework.shell import LocalShell
from test_framework.git import Git
from test_framework.cluster import Cloud

# gh output logic from determinator
from determinator import GithubOutput, write_github_output

# the environment variable containing the image tag to check for existence
IMAGE_TAG_ENV = "IMAGE_TAG"
# if running in github actions, this is the output key that will contain the latest image tag
GH_OUTPUT_KEY = "IMAGE_TAG"


def main() -> None:
    init_logging(logger=log)

    shell = LocalShell()
    git = Git(shell)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--image-name",
        "-i",
        help="The name of the image to search for",
        default="validator-testing",
    )
    parser.add_argument(
        "--variant",
        "-v",
        help="A build variant",
        action="append",
        dest="variants",
        default=[],
    )
    parser.add_argument(
        "--cloud",
        "-c",
        help="The cloud to use",
        choices=[c.value for c in Cloud],
        default=Cloud.GCP.value,
    )
    args = parser.parse_args()
    image_name = args.image_name
    cloud = Cloud(args.cloud)
    log.info(f"Using cloud: {cloud}")

    # If the IMAGE_TAG environment variable is set, check that
    if IMAGE_TAG_ENV in os.environ and os.environ[IMAGE_TAG_ENV]:
        image_tag = os.environ[IMAGE_TAG_ENV]
        if not image_exists(shell, image_name, image_tag, cloud=cloud):
            sys.exit(1)

    variants = args.variants
    log.info(f"Finding latest {image_name} image with build variants: {variants}")
    variant_prefixes = [f"{v}_" if v != "" else "" for v in variants]
    if "" not in variant_prefixes:
        variant_prefixes.append("")  # search for the default release build as well
    log.info(f"With prefixes: {variant_prefixes}")

    # Find the latest image from git history
    num_images_to_find = 1  # for the purposes of this script, this is always 1
    images = list(
        find_recent_images(
            shell, git, num_images_to_find, image_name, variant_prefixes, cloud=cloud
        )
    )
    log.info(f"Found latest images: {images}")

    # write the output to Github outputs (via stdout)
    git_sha_set = set([x.split("_")[-1] for x in images])  # trim to get the base sha
    assert len(git_sha_set) == num_images_to_find
    git_sha = git_sha_set.pop()
    log.info(f"Exporting latest image as base GIT_SHA: {git_sha}")
    latest_image_gh_output = GithubOutput(GH_OUTPUT_KEY, git_sha)
    try:
        write_github_output(latest_image_gh_output)
    except Exception as e:
        log.error(
            f"{e}\nThis may be an indication that you're running locally or the environment is not configured correctly"
        )


if __name__ == "__main__":
    main()
