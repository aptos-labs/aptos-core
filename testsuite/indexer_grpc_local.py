#!/usr/bin/env python3

from enum import Enum
import json
import platform
import time
import os
import argparse
import logging
from dataclasses import dataclass
from typing import List
from test_framework.logging import init_logging, log
from test_framework.shell import Shell, LocalShell
from test_framework.reqwest import SimpleHttpClient, HttpClient

GRPCURL_PATH = os.environ.get("GRPCURL_PATH", "grpcurl")

INDEXER_GRPC_DOCKER_COMPOSE_FILE = "docker/compose/indexer-grpc/docker-compose.yaml"
VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE = (
    "docker/compose/validator-testnet/docker-compose.yaml"
)

INDEXER_FULLNODE_REST_API_URL = "http://localhost:8080"
INDEXER_DATA_SERVICE_READINESS_URL = "http://localhost:18084/readiness"
GRPC_INDEXER_FULLNODE_URL = "localhost:50051"
GRPC_DATA_SERVICE_URL = "localhost:50052"

SHARED_DOCKER_VOLUME_NAMES = ["aptos-shared", "indexer-grpc-file-store"]

WAIT_TESTNET_START_TIMEOUT_SECS = 60
WAIT_INDEXER_GRPC_START_TIMEOUT_SECS = 60
GRPC_PROGRESS_THRESHOLD_SECS = 10


@dataclass
class SystemContext:
    shell: Shell
    http_client: HttpClient


class DockerComposeAction(Enum):
    UP = "up"
    DOWN = "down"


class Subcommand(Enum):
    START = "start"
    STOP = "stop"
    WIPE = "wipe"


class StartSubcommand(Enum):
    NO_INDEXER_GRPC = "no-indexer-grpc"


class DockerComposeError(Exception):
    def __init__(self, message="Docker Compose Error"):
        self.message = message
        super().__init__(self.message)


def run_docker_compose(
    shell: Shell,
    compose_file_path: str,
    compose_action: DockerComposeAction,
    extra_args: List[str] = [],
) -> None:
    log.info(f"Running docker-compose {compose_action.value} on {compose_file_path}")
    try:
        shell.run(
            [
                "docker-compose",
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


def start_single_validator_testnet(shell: Shell) -> None:
    run_docker_compose(
        shell, VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE, DockerComposeAction.UP
    )


def start_indexer_grpc(shell: Shell, redis_only: bool = False) -> None:
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
        shell,
        INDEXER_GRPC_DOCKER_COMPOSE_FILE,
        DockerComposeAction.UP,
        extra_args=extra_indexer_grpc_docker_args,
    )


def stop_single_validator_testnet(shell: Shell) -> None:
    run_docker_compose(
        shell, VALIDATOR_TESTNET_DOCKER_COMPOSE_FILE, DockerComposeAction.DOWN
    )


def stop_indexer_grpc(shell: Shell) -> None:
    run_docker_compose(
        shell, INDEXER_GRPC_DOCKER_COMPOSE_FILE, DockerComposeAction.DOWN
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


def wait_for_indexer_grpc_progress(shell: Shell, client: HttpClient) -> None:
    """Wait for the indexer grpc to start and try streaming from it"""
    log.info(
        f"Waiting for indexer grpc to start for {WAIT_INDEXER_GRPC_START_TIMEOUT_SECS}s"
    )
    indexer_grpc_healthcheck_up = False
    retry_secs = 5
    for _ in range(WAIT_INDEXER_GRPC_START_TIMEOUT_SECS // retry_secs):
        try:
            r = client.get(INDEXER_DATA_SERVICE_READINESS_URL)
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
        res = shell.run(
            [
                "timeout",
                f"{GRPC_PROGRESS_THRESHOLD_SECS}s",
                GRPCURL_PATH,
                "-max-msg-sz",
                "10000000",
                "-d",
                '{ "starting_version": 0 }',
                "-H",
                "x-aptos-data-authorization:dummy_token",
                "-import-path",
                "crates/aptos-protos/proto",
                "-proto",
                "aptos/indexer/v1/raw_data.proto",
                "-plaintext",
                GRPC_DATA_SERVICE_URL,
                "aptos.indexer.v1.RawData/GetTransactions",
            ],
        )
        if (
            res.exit_code == 124
        ):  # timeout exits with 124 if it reaches the end of the timeout
            indexer_grpc_data_service_up = True
            break
        time.sleep(retry_secs)

    if not indexer_grpc_data_service_up:
        if res:
            log.info(f"Stream output: {res.unwrap().decode()}")
        raise Exception(
            "Stream interrupted before reaching the end of the timeout. There might be something wrong"
        )
    log.info("Stream finished successfully")


def start(context: SystemContext, no_indexer_grpc: bool = False) -> None:
    start_single_validator_testnet(context.shell)

    # wait for progress
    latest_version = wait_for_testnet_progress(context.http_client)
    log.info(f"TESTNET STARTED: latest version @ {latest_version}")

    start_indexer_grpc(context.shell, redis_only=no_indexer_grpc)

    if not no_indexer_grpc:
        wait_for_indexer_grpc_progress(context.shell, context.http_client)


def stop(context: SystemContext) -> None:
    stop_indexer_grpc(context.shell)
    stop_single_validator_testnet(context.shell)


def wipe(context: SystemContext) -> None:
    stop(context)  # call stop() just for sanity
    context.shell.run(["docker", "volume", "rm"] + SHARED_DOCKER_VOLUME_NAMES)


def main() -> None:
    # set envs based on platform, if it's not already overriden
    if not os.environ.get("REDIS_IMAGE_REPO"):
        if platform.system() == "Darwin":
            os.environ["REDIS_IMAGE_REPO"] = "arm64v8/redis"

    parser = argparse.ArgumentParser(
        prog="Indexer GRPC Local",
        description="Spins up an indexer GRPC locally using a single validator testnet",
    )
    parser.add_argument("--verbose", "-v", action="store_true")
    subparser = parser.add_subparsers(dest="subcommand", required=True)
    start_parser = subparser.add_parser(
        Subcommand.START.value, help="Start the indexer GRPC setup"
    )
    start_parser.add_argument(
        f"--{StartSubcommand.NO_INDEXER_GRPC.value}",
        dest="no_indexer_grpc",
        action="store_true",
    )
    subparser.add_parser(Subcommand.STOP.value, help="Stop the indexer GRPC setup")
    subparser.add_parser(Subcommand.WIPE.value, help="Completely wipe the storage")
    args = parser.parse_args()
    # init logging
    init_logging(logger=log, print_metadata=True)
    if args.verbose:
        log.setLevel(logging.DEBUG)

    log.debug(f"args: {args}")

    context = SystemContext(
        shell=LocalShell(),
        http_client=SimpleHttpClient(),
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
