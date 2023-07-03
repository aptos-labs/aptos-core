#!/usr/bin/env python3

import unittest
from indexer_grpc_local import *
from test_framework.shell import FakeCommand, SpyShell, RunResult


class TestIndexerGrpcLocal(unittest.TestCase):
    def test_run_docker_compose(self):
        docker_compose_file = "docker-compose.yaml"
        extra_args = [
            "--scale",
            "banana=0",
            "--scale",
            "apple=0",
            "--scale",
            "orange=0",
        ]
        extra_args_str = " ".join(extra_args)
        shell = SpyShell(
            [
                FakeCommand(
                    f"docker-compose -f {docker_compose_file} up --detach",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    f"docker-compose -f {docker_compose_file} down",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    f"docker-compose -f {docker_compose_file} up --detach {extra_args_str}",
                    RunResult(0, b""),
                ),
            ]
        )
        run_docker_compose(shell, docker_compose_file, DockerComposeAction.UP)
        run_docker_compose(shell, docker_compose_file, DockerComposeAction.DOWN)
        run_docker_compose(
            shell,
            docker_compose_file,
            DockerComposeAction.UP,
            extra_args=extra_args,
        )
        shell.assert_commands(self)
