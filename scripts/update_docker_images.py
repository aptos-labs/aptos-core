#!/usr/bin/env python3

import os
import json
import re
import subprocess

ARCH = "amd64"
OS = "linux"

IMAGES = {
    "debian": "debian:bullseye",
    "rust": "rust:1.80.1-bullseye",
}


def update() -> int:
    script_dir = os.path.dirname(os.path.realpath(__file__))
    dockerfile_path = os.path.join(
        script_dir, "..", "docker", "builder", "docker-bake-rust-all.hcl"
    )

    update_exists = False
    with open(dockerfile_path, "r", encoding="utf-8") as f:
        dockerfile_content = f.read()

    for base_image, image_name in IMAGES.items():
        digest = None
        current_digest = None
        regex = rf'{re.escape(base_image)} = "docker-image://{re.escape(image_name)}@([^"]+)"'

        print(f"Update {image_name}")
        # Note space before {{ in --format is important.
        try:
            result = subprocess.run(
                [
                    "docker",
                    "buildx",
                    "imagetools",
                    "inspect",
                    image_name,
                    "--format",
                    "{{json .Manifest.Digest}}",
                ],
                check=True,
                capture_output=True,
                text=True,
            )
            digest = result.stdout.strip().strip('"')
        except subprocess.CalledProcessError:
            print(f"Unable to find digest for {image_name}")
            continue

        if not digest:
            print(f"Unable to find digest for {image_name}")
            continue

        print(f"Found digest for {image_name}: {digest}")
        match = re.search(regex, dockerfile_content)
        if match:
            current_digest = match.group(1)

        if current_digest is None:
            print(f"Unable to find current_digest for {image_name}")
            continue

        if current_digest == digest:
            print(f"{image_name} is up to date: {current_digest} = {digest}")
            continue

        print(f"Found update for {image_name}: {current_digest} -> {digest}")
        dockerfile_content = re.sub(
            regex,
            f'{base_image} = "docker-image://{image_name}@{digest}"',
            dockerfile_content,
        )
        update_exists = True

    if update_exists:
        with open(dockerfile_path, "w", encoding="utf-8") as f:
            f.write(dockerfile_content)

    return update_exists


def write_github_output(key, value) -> None:
    print(f"GITHUB_OUTPUT: {key}={value}")
    try:
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(f"{key}={value}\n")
    except KeyError:
        print("GITHUB_OUTPUT environment variable not set")
        exit()


if __name__ == "__main__":
    write_github_output("NEED_UPDATE", update())
    exit()
