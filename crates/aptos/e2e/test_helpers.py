# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import logging
import os
import pathlib
import subprocess
import traceback
import typing

from dataclasses import dataclass

from common import AccountInfo

LOG = logging.getLogger(__name__)

WORKING_DIR_IN_CONTAINER = "/tmp"

# We pass this class into all test functions to help with calling the CLI,
# collecting output, and accessing common info.
@dataclass
class RunHelper:
    host_working_directory: str
    image_repo: str
    image_tag: str
    cli_path: str
    passed_tests: typing.List[str]
    failed_tests: typing.List[str]

    def __init__(self, host_working_directory, image_repo, image_tag, cli_path):
        if image_tag and cli_path:
            raise RuntimeError("Cannot specify both image_tag and cli_path")
        if not (image_tag or cli_path):
            raise RuntimeError("Must specify one of image_tag and cli_path")
        self.host_working_directory = host_working_directory
        self.image_repo = image_repo
        self.image_tag = image_tag
        self.cli_path = cli_path
        self.passed_tests = []
        self.failed_tests = []

    def build_image_name(self):
        return f"{self.image_repo}aptoslabs/tools:{self.image_tag}"

    # This function lets you pass call the CLI like you would normally, but really it is
    # calling the CLI in a docker container and mounting the host working directory such
    # that the container will write it results out to that directory. That way the CLI
    # state / configuration is preserved between test cases.
    def run_command(self, test_name, command, *args, **kwargs):
        LOG.info(f"Running test: {test_name}")

        # Build command.
        if self.image_tag:
            full_command = [
                "docker",
                "run",
                "--rm",
                "--network",
                "host",
                "-i",
                "-v",
                f"{self.host_working_directory}:{WORKING_DIR_IN_CONTAINER}",
                "--workdir",
                WORKING_DIR_IN_CONTAINER,
                self.build_image_name(),
            ] + command
        else:
            full_command = [self.cli_path] + command[1:]
        LOG.debug(f"Running command: {full_command}")

        # Create the output directory if necessary.
        out_path = os.path.join(self.host_working_directory, "out")
        pathlib.Path(out_path).mkdir(exist_ok=True)

        # Write the command we're going to run to file.
        with open(os.path.join(out_path, f"{test_name}.command"), "w") as f:
            f.write(" ".join(command))

        # Run command.
        try:
            # If we're using a local CLI, set the working directory for subprocess.run.
            if self.cli_path:
                kwargs["cwd"] = self.host_working_directory
            result = subprocess.run(
                full_command,
                *args,
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                universal_newlines=True,
                **kwargs,
            )
            LOG.info(f"Test passed: {test_name}")
            self.passed_tests.append(test_name)

            out = result
        except Exception as e:
            LOG.warn(f"Test failed: {test_name}")
            self.failed_tests.append(test_name)

            # Write the exception to file.
            with open(os.path.join(out_path, f"{test_name}.exception"), "w") as f:
                f.write(
                    "".join(
                        traceback.format_exception(
                            etype=type(e), value=e, tb=e.__traceback__
                        )
                    )
                )

            # Fortunately the result and exception of subprocess.run both have the
            # stdout and stderr attributes on them.
            out = e

        LOG.debug(f"Stdout: {out.stdout}")
        LOG.debug(f"Stderr: {out.stderr}")

        # Write stdout and stderr to file.
        with open(os.path.join(out_path, f"{test_name}.stdout"), "w") as f:
            f.write(out.stdout)
        with open(os.path.join(out_path, f"{test_name}.stderr"), "w") as f:
            f.write(out.stderr)

        return out

    # If image_Tag is set, pull the test CLI image. We don't technically have to do
    # this separately but it makes the steps clearer. Otherwise, cli_path must be
    # set, in which case we ensure the file is there.
    def prepare(self):
        if self.image_tag:
            command = ["docker", "pull", self.build_image_name()]
            LOG.debug(f"Running command: {command}")
            output = subprocess.check_output(command)
            LOG.debug(f"Output: {output}")
        else:
            if not os.path.isfile(self.cli_path):
                raise RuntimeError(f"CLI not found at path: {self.cli_path}")

    # Get the account info of the account created by test_init.
    def get_account_info(self):
        path = os.path.join(self.host_working_directory, ".aptos", "config.yaml")
        with open(path) as f:
            content = f.read().splitlines()
        # To avoid using external deps we parse the file manually.
        private_key = None
        public_key = None
        account_address = None
        for line in content:
            if "private_key: " in line:
                private_key = line.split("private_key: ")[1].replace('"', "")
            if "public_key: " in line:
                public_key = line.split("public_key: ")[1].replace('"', "")
            if "account: " in line:
                account_address = line.split("account: ")[1].replace('"', "")
        if not private_key or not public_key or not account_address:
            raise RuntimeError(f"Failed to parse {path} to get account info")
        return AccountInfo(
            private_key=private_key,
            public_key=public_key,
            account_address=account_address,
        )
