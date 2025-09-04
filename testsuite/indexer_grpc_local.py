#!/usr/bin/env python3

import argparse
import json
import logging
import os
import platform
import shutil
import subprocess
import time
from dataclasses import dataclass
from enum import Enum
from typing import List, Optional, Sequence

from test_framework.logging import init_logging, log
from test_framework.reqwest import HttpClient, SimpleHttpClient
from test_framework.shell import LocalShell, RunResult, Shell

GRPCURL_PATH = os.environ.get("GRPCURL_PATH", "grpcurl")

INDEXER_GRPC_DOCKER_COMPOSE_FILE = "docker/compose/indexer-grpc/docker-compose.yaml"
INDEXER_GRPC_DATA_SERVICE_CERT_FILE = (
    "docker/compose/indexer-grpc/data-service-grpc-server.crt"
)
INDEXER_GRPC_DATA_SERVICE_KEY_FILE = (
    "docker/compose/indexer-grpc/data-service-grpc-server.key"
)
VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE = (
    "docker/compose/validator-testnet/docker-compose.yaml"
)

INDEXER_FULLNODE_REST_API_URL = "http://localhost:8080"
INDEXER_DATA_SERVICE_READINESS_URL = "http://localhost:18084/readiness"
GRPC_INDEXER_FULLNODE_URL = "localhost:50051"
GRPC_DATA_SERVICE_NON_TLS_URL = "localhost:50052"
GRPC_DATA_SERVICE_TLS_URL = "localhost:50053"

GRPC_IS_READY_MESSAGE = f"""
    ======================================
    Transaction Stream Service(indexer grpc) is ready to serve!

    You can use grpcurl to test it out:

    - For non-TLS:
        grpcurl -plaintext -d '{{ "starting_version": 0 }}' \\
            -H "x-velor-data-authorization:dummy_token" \\
            {GRPC_DATA_SERVICE_NON_TLS_URL} velor.indexer.v1.RawData/GetTransactions
    - For TLS:
        grpcurl -insecure -d '{{ "starting_version": 0 }}' \\
            -H "x-velor-data-authorization:dummy_token" \\
            {GRPC_DATA_SERVICE_TLS_URL} velor.indexer.v1.RawData/GetTransactions
    ======================================
"""

SHARED_DOCKER_VOLUME_NAMES = ["velor-shared", "indexer-grpc-file-store"]

WAIT_TESTNET_START_TIMEOUT_SECS = 60
WAIT_INDEXER_GRPC_START_TIMEOUT_SECS = 60
GRPC_PROGRESS_THRESHOLD_SECS = 10


@dataclass
class SystemContext:
    shell: Shell
    http_client: HttpClient
    run_docker_as_root: bool

    def run_docker_command(
        self,
        args: Sequence[str],
        pre_args: Optional[Sequence[str]] = None,
        stream_output: bool = False,
    ) -> RunResult:
        base = ["sudo"] if self.run_docker_as_root else []
        command = (list(pre_args) if pre_args else []) + base + ["docker"] + list(args)
        return self.shell.run(command, stream_output=stream_output)

    def create_grpc_testing_certificates_if_absent(self) -> None:
        # Check if the certificates are already present
        if os.path.isfile(INDEXER_GRPC_DATA_SERVICE_CERT_FILE) and os.path.isfile(
            INDEXER_GRPC_DATA_SERVICE_KEY_FILE
        ):
            return
        # If not, create them
        log.info("Creating grpc testing certificates")
        command = [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:4096",
            "-subj",
            "/C=US/ST=CA/L=SF/O=Testing/CN=www.testing.com",
            "-keyout",
            INDEXER_GRPC_DATA_SERVICE_KEY_FILE,
            "-out",
            INDEXER_GRPC_DATA_SERVICE_CERT_FILE,
            "-days",
            "365",
            "-nodes",
        ]
        self.shell.run(command)


class DockerComposeAction(Enum):
    UP = "up"
    DOWN = "down"


class Subcommand(Enum):
    START = "start"
    STOP = "stop"
    WIPE = "wipe"


class DockerComposeError(Exception):
    def __init__(self, message="Docker Compose Error"):
        self.message = message
        super().__init__(self.message)


def run_docker_compose(
    context: SystemContext,
    compose_file_path: str,
    compose_action: DockerComposeAction,
    extra_args: List[str] = [],
) -> None:
    log.info(f"Running docker compose {compose_action.value} on {compose_file_path}")
    try:
        context.run_docker_command(
            [
                "compose",
                "-f",
                compose_file_path,
                compose_action.value,
            ]
            + (["--detach"] if compose_action == DockerComposeAction.UP else [])
            + extra_args,
            stream_output=True,
        )
    except Exception as e:
        if "No such file or directory" in str(e):
            raise DockerComposeError("Failed to find the compose file") from e
        else:
            raise e


def start_single_validator_testnet(context: SystemContext) -> None:
    run_docker_compose(
        context, VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE, DockerComposeAction.UP
    )


def start_indexer_grpc(context: SystemContext, redis_only: bool = False) -> None:
    context.create_grpc_testing_certificates_if_absent()
    extra_indexer_grpc_docker_args = []
    if redis_only:
        extra_indexer_grpc_docker_args = [
            "--scale",
            "indexer-grpc-cache-worker=0",
            "--scale",
            "indexer-grpc-file-store=0",
            "--scale",
            "indexer-grpc-data-service=0",
        ]

    run_docker_compose(
        context,
        INDEXER_GRPC_DOCKER_COMPOSE_FILE,
        DockerComposeAction.UP,
        extra_args=extra_indexer_grpc_docker_args,
    )


def stop_single_validator_testnet(context: SystemContext) -> None:
    run_docker_compose(
        context, VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE, DockerComposeAction.DOWN
    )


def stop_indexer_grpc(context: SystemContext) -> None:
    run_docker_compose(
        context, INDEXER_GRPC_DOCKER_COMPOSE_FILE, DockerComposeAction.DOWN
    )


def wait_for_testnet_progress(client: HttpClient) -> int:
    """Wait for the testnet to start and return the latest version"""
    r = None
    ledger_version_key = "ledger_version"
    for _ in range(WAIT_TESTNET_START_TIMEOUT_SECS):
        try:
            r = client.get(INDEXER_FULLNODE_REST_API_URL + "/v1")
            if r.status_code == 200:
                response = json.loads(r.text)
                log.debug(f"LedgerInfo: {response}")
                version = int(response[ledger_version_key])
                if version > 0:  # we're making some progress
                    return version
        except KeyError as e:
            log.info(f"Key not found: {e}")
        except Exception as e:
            log.info(f"Exception: {e}")
        time.sleep(5)

    raise Exception("Testnet failed to start within timeout period")


def wait_for_indexer_grpc_progress(context: SystemContext) -> None:
    """Wait for the indexer grpc to start and try streaming from it"""
    log.info(
        f"Waiting for indexer grpc to start for {WAIT_INDEXER_GRPC_START_TIMEOUT_SECS}s"
    )
    indexer_grpc_healthcheck_up = False
    retry_secs = 5
    for _ in range(WAIT_INDEXER_GRPC_START_TIMEOUT_SECS // retry_secs):
        try:
            r = context.http_client.get(INDEXER_DATA_SERVICE_READINESS_URL)
            if r.status_code == 200:
                log.info("Indexer grpc data service is up")
                indexer_grpc_healthcheck_up = True
                break
        except Exception as e:
            log.info(f"Exception: {e}")
        time.sleep(retry_secs)

    if not indexer_grpc_healthcheck_up:
        raise Exception("Indexer grpc failed to start within timeout period")

    indexer_grpc_data_service_up = False
    log.info(
        f"Attempting to stream from indexer grpc for {GRPC_PROGRESS_THRESHOLD_SECS}s"
    )
    res = None
    for _ in range(GRPC_PROGRESS_THRESHOLD_SECS // retry_secs):
        try:
            res = context.shell.run(
                [
                    GRPCURL_PATH,
                    "-max-msg-sz",
                    "10000000",
                    "-d",
                    '{ "starting_version": 0 }',
                    "-H",
                    "x-velor-data-authorization:dummy_token",
                    "-import-path",
                    "protos/proto",
                    "-proto",
                    "velor/indexer/v1/raw_data.proto",
                    "-plaintext",
                    GRPC_DATA_SERVICE_NON_TLS_URL,
                    "velor.indexer.v1.RawData/GetTransactions",
                ],
                timeout_secs=GRPC_PROGRESS_THRESHOLD_SECS,
            )
        except subprocess.TimeoutExpired:
            # If it timed out, great! That means the command was still running after
            # 10 seconds, implying it connected and was streaming. If it exited prior
            # to the timeout then we know something went wrong.
            indexer_grpc_data_service_up = True
            break
        time.sleep(retry_secs)

    if not indexer_grpc_data_service_up:
        if res:
            log.info(f"Stream output: {res.unwrap().decode()}")
        raise RuntimeError(
            "Stream interrupted before reaching the end of the timeout. There might be something wrong"
        )
    log.info("Stream finished successfully")


def start(context: SystemContext, no_indexer_grpc: bool = False) -> None:
    start_single_validator_testnet(context)

    # wait for progress
    latest_version = wait_for_testnet_progress(context.http_client)
    log.info(f"TESTNET STARTED: latest version @ {latest_version}")

    start_indexer_grpc(context, redis_only=no_indexer_grpc)

    if not no_indexer_grpc:
        wait_for_indexer_grpc_progress(context)
        log.info(GRPC_IS_READY_MESSAGE)


def stop(context: SystemContext) -> None:
    stop_indexer_grpc(context)
    stop_single_validator_testnet(context)


def wipe(context: SystemContext) -> None:
    stop(context)  # call stop() just for sanity

    context.run_docker_command(["volume", "rm"] + SHARED_DOCKER_VOLUME_NAMES)


def parse_args():
    parser = argparse.ArgumentParser(
        prog="Indexer GRPC Local",
        description=(
            "Spins up an indexer GRPC (Transaction Stream Service) locally "
            "using a single validator testnet"
        ),
        # This causes argparse to raise an error for undefined flags
        fromfile_prefix_chars="@",
    )

    parser.add_argument("--verbose", "-v", action="store_true")
    parser.add_argument(
        "--run-docker-as-root",
        action="store_true",
        help="If set, prefix 'sudo' to all docker commands",
    )

    subparser = parser.add_subparsers(dest="subcommand", required=True)

    start_parser = subparser.add_parser(
        Subcommand.START.value, help="Start the indexer GRPC setup"
    )
    start_parser.add_argument(
        "--no-indexer-grpc",
        action="store_true",
    )

    subparser.add_parser(Subcommand.STOP.value, help="Stop the indexer GRPC setup")

    subparser.add_parser(Subcommand.WIPE.value, help="Completely wipe the storage")

    return parser.parse_args()


def check_system(context: SystemContext) -> None:
    # Check that docker is installed running.
    if not shutil.which("docker"):
        raise RuntimeError("Docker is not installed or not in PATH")

    # Check that docker is running.
    result = context.run_docker_command(["info"])
    if not result.succeeded():
        log.debug(f"Output of 'docker info': {result.output_str()}")
        raise RuntimeError(
            "Docker is installed but doesn't seem to be running or the user doesn't "
            "have permission to interact with it"
        )

    # Check that docker compose v2 is available.
    result = context.run_docker_command(["compose", "version", "--short"])
    if not result.succeeded():
        log.debug(f"Output of 'docker compose version': {result.output_str()}")
        raise RuntimeError("Docker Compose is not available")

    if not result.output_str().startswith("2"):
        raise RuntimeError(
            f"This script only works with Docker Compose v2 but you have version "
            f"{result.output_str()} installed"
        )

    # Check that grpcurl is installed.
    if not shutil.which(GRPCURL_PATH):
        raise RuntimeError(f"{GRPCURL_PATH} is not installed or not in PATH")

    # Check that openssl is installed.
    if not shutil.which("openssl"):
        raise RuntimeError("openssl is not installed or not in PATH")


def main() -> None:
    # Change to the root of velor-core.
    abspath = os.path.abspath(__file__)
    dname = os.path.dirname(abspath)
    os.chdir(dname)
    os.chdir("..")

    args = parse_args()

    # Init logging.
    init_logging(logger=log, print_metadata=True)
    if args.verbose:
        log.setLevel(logging.DEBUG)

    log.debug(f"Args: {args}")

    context = SystemContext(
        shell=LocalShell(),
        http_client=SimpleHttpClient(),
        run_docker_as_root=args.run_docker_as_root,
    )

    # Check that the system is in a good state.
    check_system(context)

    if platform.system() == "Darwin" and platform.processor().startswith("arm"):
        # If we're on an ARM Mac, use the amd64 Redis image. On some ARM Macs the ARM
        # Redis image doesn't work so we use the amd64 image for now. See more here:
        # https://github.com/velor-chain/velor-core/issues/9878
        if not os.environ.get("REDIS_IMAGE_REPO"):
            os.environ["REDIS_IMAGE_REPO"] = "amd64/redis"
            log.info(
                "Detected ARM Mac and REDIS_IMAGE_REPO was not set, setting it to "
                "amd64/redis"
            )

        # For all other images use amd64, since we don't publish ARM builds.
        if not os.environ.get("DOCKER_DEFAULT_PLATFORM"):
            os.environ["DOCKER_DEFAULT_PLATFORM"] = "linux/amd64"
            log.info(
                "Detected ARM Mac and DOCKER_DEFAULT_PLATFORM was not set, setting it "
                "to linux/amd64"
            )

    subcommand = Subcommand(args.subcommand)

    if subcommand == Subcommand.START:
        start(
            context,
            args.no_indexer_grpc,
        )
    elif subcommand == Subcommand.STOP:
        stop(context)
        log.info("To wipe all data, run: $ ./testsuite/indexer_grpc_local.py wipe")
        log.info("To start again, run: $ ./testsuite/indexer_grpc_local.py start")
    elif subcommand == Subcommand.WIPE:
        wipe(context)


if __name__ == "__main__":
    main()
