#!/usr/bin/env python3

import os
import json
import re
import subprocess

ARCH = "amd64"
OS = "linux"

IMAGES = {
    "debian-base": "debian:bullseye",
    "rust-base": "rust:1.66.1-bullseye",
}


def update() -> int:
    script_dir = os.path.dirname(os.path.realpath(__file__))
    dockerfile_path = os.path.join(script_dir, "..", "docker", "rust-all.Dockerfile")

    exit_code = 1  # 0 = an update exists, 1 = an update does not exist

    for base_image, image_name in IMAGES.items():
        manifest = None
        digest = None
        current_digest = None
        regex = f"FROM [\S]+ AS {base_image}"

        print(f"Update {image_name}")
        manifest_inspect = subprocess.check_output(["docker", "manifest", "inspect", image_name])
        manifest = json.loads(manifest_inspect)

        if manifest == None:
            print(f"Unable to find manifest for {image_name}")
            continue

        for m in manifest["manifests"]:
            if m["platform"]["architecture"] == ARCH and m["platform"]["os"] == OS:
                digest = m["digest"]
                break

        if digest == None:
            print(f"Unable to find digest for {image_name}")
            continue

        print(f"Found digest for {image_name}: {digest}")
        with open(dockerfile_path, "r") as f:
            dockerfile_content = f.read()

        for line in dockerfile_content.splitlines():
            if re.match(regex, line):
                current_digest = line.split()[1].split("@")[1]
                break
            
        if current_digest == None:
            print(f"Unable to find current_digest for {image_name}")
            continue

        if current_digest == digest:
            print(f"{image_name} is up to date: {current_digest} = {digest}")
            continue

        print(f"Found update for {image_name}: {current_digest} -> {digest}")
        dockerfile_content = re.sub(regex, f"FROM {image_name}@{digest} AS {base_image}", dockerfile_content)

        with open(dockerfile_path, "w") as f:
            f.write(dockerfile_content)

        exit_code = 0

    return exit_code


if __name__ == "__main__":
    exit(update())
