#!/usr/bin/env python3

# This script is used to find the latest built image tag. It has two modes where it can be used:
# 1. If the IMAGE_TAG environment variable is set, it will simply check that the image exists.
# 2. If the IMAGE_TAG environment variable is not set, it will find the latest image tag from git history,
#    assuming images are tagged with the git commit hash.

from forge import find_recent_images, image_exists
from forge_wrapper_core.shell import LocalShell
from forge_wrapper_core.git import Git

import os
import sys

IMAGE_NAME = "aptos/validator"
IMAGE_TAG_ENV = "IMAGE_TAG"


def main():
    shell = LocalShell()
    git = Git(shell)

    # If the IMAGE_TAG environment variable is set, check that
    if IMAGE_TAG_ENV in os.environ:
        image_tag = os.environ[IMAGE_TAG_ENV]
        if not image_exists(shell, IMAGE_NAME, image_tag):
            sys.exit(1)

    # Find the latest image from git history
    images = list(find_recent_images(shell, git, 1, IMAGE_NAME))
    print(images[0])


if __name__ == "__main__":
    main()
