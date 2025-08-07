from __future__ import annotations

import asyncio
import difflib
import json
import logging
import os
import random
import re
import resource
import sys
import textwrap
from contextlib import contextmanager
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
from typing import (
    Any,
    Callable,
    Generator,
    Iterator,
    List,
    Mapping,
    Optional,
    Sequence,
    Set,
    Tuple,
    TypedDict,
    Union,
)
from urllib.parse import ParseResult, urlencode, urlunparse
from urllib.parse import quote as urlquote

from test_framework.cluster import Cloud, ForgeCluster, ForgeJob, find_forge_cluster
from test_framework.filesystem import Filesystem, LocalFilesystem
from test_framework.git import Git
from test_framework.logging import init_logging, log
from test_framework.process import Processes, SystemProcesses
from test_framework.shell import LocalShell, Shell
from test_framework.time import SystemTime, Time

# map of build variant (e.g. cargo profile and feature flags)
BUILD_VARIANT_TAG_PREFIX_MAP = {
    "performance": "performance",
    "failpoints": "failpoints",
    "indexer": "indexer",
    "release": "",  # the default release profile has no tag prefix
}

VALIDATOR_IMAGE_NAME = "validator"
VALIDATOR_TESTING_IMAGE_NAME = "validator-testing"
FORGE_IMAGE_NAME = "forge"
ECR_REPO_PREFIX = "aptos"

DEFAULT_CONFIG = "forge-wrapper-config"
DEFAULT_CONFIG_KEY = "forge-wrapper-config.json"

FORGE_TEST_RUNNER_TEMPLATE_PATH = "forge-test-runner-template.yaml"

MULTIREGION_KUBECONFIG_DIR = "/etc/multiregion-kubeconfig"
MULTIREGION_KUBECONFIG_PATH = f"{MULTIREGION_KUBECONFIG_DIR}/kubeconfig"
GAR_REPO_NAME = "us-docker.pkg.dev/aptos-registry/docker"


@dataclass
class RunResult:
    exit_code: int
    output: bytes

    def unwrap(self) -> bytes:
        if not self.succeeded():
            raise Exception(self.output.decode("utf-8"))
        return self.output

    def succeeded(self) -> bool:
        return self.exit_code == 0


def get_prompt_answer(prompt: str, answer: Optional[str] = None) -> bool:
    """Get a yes/no answer from the user, or use the default answer if provided."""
    if not answer and not os.getenv("CI"):
        answer = input(f"{prompt} (y/n) ").strip().lower()
    return answer in ("y", "yes", "yeet", "yessir", "si", "true")


def install_dependency(dependency: str) -> None:
    log.info(f"{dependency} is not currently installed")
    answer = os.getenv("FORGE_INSTALL_DEPENDENCIES") or os.getenv("CI")
    if get_prompt_answer("Would you like to install it now?", answer):
        shell = LocalShell()
        shell.run(["poetry", "install", dependency], stream_output=True).unwrap()
    else:
        log.fatal(f"Please install click (poetry install {dependency}) and try again")


try:
    import click
except ImportError:
    install_dependency("click")
    import click

try:
    import psutil
except ImportError:
    install_dependency("psutil")
    import psutil


@click.group()
@click.option(
    "--log-metadata/--no-log-metadata",
    default=True,
)
def main(log_metadata: bool) -> None:
    init_logging(logger=log, print_metadata=log_metadata)


def envoption(name: str, default: Optional[Any] = None) -> Any:
    return click.option(
        f"--{name.lower().replace('_', '-')}",
        default=lambda: os.getenv(name, default() if callable(default) else default),
        show_default=True,
    )


# o11y resources
GRAFANA_BASE_URL = "https://aptoslabs.grafana.net/d/overview/overview?orgId=1&refresh=10s&var-Datasource=VictoriaMetrics%20Global%20%28Non-mainnet%29&var-BigQuery=Google%20BigQuery"

# helm chart default override values
HELM_CHARTS = ["aptos-node", "aptos-genesis"]


class ForgeState(Enum):
    RUNNING = "RUNNING"
    PASS = "PASS"
    SOFT_FAIL = "SOFT_FAIL"
    FAIL = "FAIL"
    SKIP = "SKIP"
    EMPTY = "EMPTY"


class ForgeResult:
    def __init__(self):
        self.state: ForgeState = ForgeState.EMPTY
        self.output: str = ""
        self.debugging_output: str = ""
        self._start_time: Optional[datetime] = None
        self._end_time: Optional[datetime] = None

    @property
    def start_time(self) -> datetime:
        assert self._start_time is not None, "start_time is not set"
        return self._start_time

    @property
    def end_time(self) -> datetime:
        assert self._end_time is not None, "end_time is not set"
        return self._end_time

    @property
    def duration(self) -> float:
        return (self.end_time - self.start_time).total_seconds()

    @classmethod
    def from_args(cls, state: ForgeState, output: str) -> ForgeResult:
        result = cls()
        result.state = state
        result.output = output
        return result

    @classmethod
    def empty(cls) -> ForgeResult:
        return cls.from_args(ForgeState.EMPTY, "")

    @classmethod
    @contextmanager
    def with_context(
        cls, context: "ForgeContext"
    ) -> Generator["ForgeResult", None, None]:
        result = cls()
        result.state = ForgeState.RUNNING
        result._start_time = context.time.now()
        try:
            yield result
            result.set_debugging_output(
                dump_forge_state(
                    context.shell,
                    context.forge_namespace,
                    context.forge_cluster.kubeconf,
                )
            )
        except Exception as e:
            result.set_state(ForgeState.FAIL)
            result.set_debugging_output(
                "{}\n{}\n".format(
                    str(e),
                    dump_forge_state(
                        context.shell,
                        context.forge_namespace,
                        context.forge_cluster.kubeconf,
                    ),
                )
            )
        result._end_time = context.time.now()
        if result.state not in (
            ForgeState.PASS,
            ForgeState.SOFT_FAIL,
            ForgeState.FAIL,
            ForgeState.SKIP,
        ):
            raise Exception("Forge result never entered terminal state")
        if result.output is None:
            raise Exception("Forge result didnt record output")

    def set_state(self, state: ForgeState) -> None:
        log.info(f"Setting state to {state.value}")
        self.state = state

    def set_output(self, output: str) -> None:
        self.output = output

    def set_debugging_output(self, output: str) -> None:
        self.debugging_output = output

    def format(self, context: ForgeContext) -> str:
        output_lines: List[str] = []
        if not self.succeeded():
            output_lines.append(self.debugging_output)
        output_lines.extend(
            [
                f"Forge output: {self.output}",
                f"Forge {self.state.value.lower()}ed",
            ]
        )
        if self.state == ForgeState.FAIL and self.duration > 3600:
            output_lines.append(
                "Forge took longer than 1 hour to run. This can cause the job to"
                " fail even when the test is successful because of gcp + github"
                " auth expiration. If you think this is the case please check the"
                " GCP_AUTH_DURATION in the github workflow."
            )
        return "\n".join(output_lines)

    def succeeded(self) -> bool:
        return self.state == ForgeState.PASS

    def is_hard_failure(self) -> bool:
        return self.state == ForgeState.FAIL


@dataclass
class SystemContext:
    shell: Shell
    filesystem: Filesystem
    processes: Processes
    time: Time


@dataclass
class ForgeContext:
    shell: Shell
    filesystem: Filesystem
    processes: Processes
    time: Time

    # forge cluster options
    forge_namespace: str
    forge_args: Sequence[str]

    forge_image_tag: str
    image_tag: str
    upgrade_image_tag: str
    forge_cluster: ForgeCluster
    forge_test_suite: str
    forge_username: str
    forge_blocking: bool
    forge_retain_debug_logs: str
    forge_junit_xml_path: Optional[str]

    github_actions: str
    github_job_url: Optional[str]

    # aws related options
    aws_account_num: Optional[str]
    aws_region: Optional[str]

    # gcp related options
    gcp_project: Optional[str] = None
    gcp_zone: Optional[str] = None

    # the default cloud is AWS
    cloud: Cloud = Cloud.AWS

    def report(self, result: ForgeResult, outputs: List[ForgeFormatter]) -> None:
        for formatter in outputs:
            output = formatter.format(self, result)
            log.info(f"=== Start {formatter} ===")
            log.info(output)
            log.info(f"=== End {formatter} ===")
            self.filesystem.write(formatter.filename, output.encode())

    @property
    def forge_chain_name(self) -> str:
        forge_chain_name = self.forge_cluster.name.lstrip("aptos-")
        if "forge" not in forge_chain_name:
            forge_chain_name += "net"
        return forge_chain_name


@dataclass
class ForgeFormatter:
    filename: str
    _format: Callable[[ForgeContext, ForgeResult], str]

    def format(self, context: ForgeContext, result: ForgeResult) -> str:
        return self._format(context, result)

    def __str__(self) -> str:
        return self.filename


def format_report(context: ForgeContext, result: ForgeResult) -> str:
    report_lines = []
    recording = False
    error_buffer = []
    error_length = 10
    for line in result.output.splitlines():
        if line in ("====json-report-begin===", "====json-report-end==="):
            recording = not recording
        elif recording:
            report_lines.append(line)
        else:
            if len(error_buffer) == error_length and not report_lines:
                error_buffer.pop(0)
            error_buffer.append(line)
    report_output = "\n".join(report_lines)
    error_output = "\n".join(error_buffer)
    debugging_appendix = "Trailing Log Lines:\n{}\nDebugging output:\n{}".format(
        error_output, result.debugging_output
    )
    if not report_lines:
        return "Forge test runner terminated:\n{}".format(debugging_appendix)
    report_text = None
    try:
        report_text = json.loads(report_output).get("text")
    except Exception as e:
        return "Forge report malformed: {}\n{}\n{}".format(
            e, repr(report_output), debugging_appendix
        )
    if not report_text:
        return "Forge report text empty. See test runner output.\n{}".format(
            debugging_appendix
        )
    else:
        if result.state in (ForgeState.FAIL, ForgeState.SOFT_FAIL):
            return "{}\n{}".format(report_text, debugging_appendix)
        return report_text


def get_dashboard_link(
    forge_namespace: str,
    forge_chain_name: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    if time_filter is True:
        grafana_time_filter = "&refresh=10s&from=now-15m&to=now"
    elif isinstance(time_filter, tuple):
        start_ms = int(time_filter[0].timestamp()) * 1000
        end_ms = int(time_filter[1].timestamp()) * 1000
        grafana_time_filter = f"&from={start_ms}&to={end_ms}"
    else:
        raise Exception(f"Invalid refresh argument: {time_filter}")

    return (
        f"{GRAFANA_BASE_URL}&var-namespace={forge_namespace}&var-metrics_source=All"
        f"&var-chain_name={forge_chain_name}{grafana_time_filter}"
    )


class ContainerName(str, Enum):
    Validator = "validator"
    FullNode = "fullnode"


def get_cpu_profile_link(
    container_name: ContainerName,
    forge_namespace: str,
    start_time: datetime | None = None,
    end_time: datetime | None = None,
) -> str:
    base_url = "https://grafana.aptoslabs.com/a/grafana-pyroscope-app/profiles-explorer"
    start_timestamp = str(int(start_time.timestamp())) if start_time else "now-1h"
    end_timestamp = str(int(end_time.timestamp())) if end_time else "now"

    query_params = [
        ("from", start_timestamp),
        ("until", end_timestamp),
        ("maxNodes", "16384"),
        ("explorationType", "flame-graph"),
        ("var-serviceName", "ebpf/forge"),
        ("var-filters", f"namespace|=|{forge_namespace}"),
        ("var-filters", f"pod|=~|.*{container_name.value}.*"),
    ]
    encoded_params = urlencode(query_params)

    return f"{base_url}?{encoded_params}"


def milliseconds(timestamp: datetime) -> int:
    return int(timestamp.timestamp()) * 1000


def apply_humio_time_filter(
    urlparts: Mapping[str, Union[str, bool, int]],
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> Mapping:
    if time_filter is True:
        urlparts = {
            **urlparts,
            "live": "true",
            "start": "30m",
        }
    elif isinstance(time_filter, tuple):
        start_ms = milliseconds(time_filter[0])
        end_ms = milliseconds(time_filter[1])
        urlparts = {
            **urlparts,
            "live": "false",
            "start": start_ms,
            "end": end_ms,
        }
    else:
        raise Exception(f"Invalid refresh argument: {time_filter}")
    return urlparts


def get_humio_link_for_test_runner_logs(
    forge_namespace: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    """Get a link to the forge test runner logs in humio for a given test run in a given namespace"""
    columns = [
        {
            "type": "field",
            "fieldName": "@timestamp",
            "format": "timestamp",
            "width": 180,
        },
        {
            "type": "link",
            "openInNewBrowserTab": "***",
            "style": "button",
            "hrefTemplate": 'https://github.com/aptos-labs/aptos-core/pull/{{fields["github_pr"]}}',
            "textTemplate": '{{fields["github_pr"]}}',
            "header": "Forge PR",
            "width": 79,
        },
        {"type": "field", "fieldName": "k8s.namespace", "format": "text", "width": 104},
        {"type": "field", "fieldName": "message", "format": "text", "width": 3760},
    ]
    urlparts = {
        "query": (
            "$forgeLogs(validator_instance=*)"
            f" | {forge_namespace}"
            ' | "k8s.labels.app.kubernetes.io/name" = forge'
        ),
        "widgetType": "list-view",
        "columns": json.dumps(columns),
        "newestAtBottom": "true",
        "showOnlyFirstLine": "false",
    }
    urlparts = apply_humio_time_filter(urlparts, time_filter)
    query = urlencode(urlparts)
    return urlunparse(
        ParseResult("https", "cloud.us.humio.com", "/k8s/search", "", query, "")
    )


def get_humio_link_for_node_logs(
    forge_namespace: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    """Get a link to the node logs in humio for a given test run in a given namespace"""
    query = (
        f"$forgeLogs(validator_instance=*) |\n"
        f'    "k8s.namespace" = "{forge_namespace}" // filters on namespace which contains validator logs\n'
        f"   OR  // remove either side of the OR operator to only display validator or forge-runner logs\n"
        f'    ("k8s.namespace"=default AND "k8s.labels.forge-namespace" = "{forge_namespace}") // filters on specific forge-runner pod in default namespace\n'
    )
    columns = [
        {
            "type": "field",
            "fieldName": "@timestamp",
            "format": "timestamp",
            "width": 180,
        },
        {"type": "field", "fieldName": "level", "format": "text", "width": 54},
        {
            "type": "link",
            "openInNewBrowserTab": "***",
            "style": "button",
            "hrefTemplate": 'https://github.com/aptos-labs/aptos-core/pull/{{fields["github_pr"]}}',
            "textTemplate": '{{fields["github_pr"]}}',
            "header": "Forge PR",
            "width": 79,
        },
        {"type": "field", "fieldName": "k8s.namespace", "format": "text", "width": 104},
        {"type": "field", "fieldName": "k8s.pod_name", "format": "text", "width": 126},
        {
            "type": "field",
            "fieldName": "k8s.container_name",
            "format": "text",
            "width": 85,
        },
        {"type": "field", "fieldName": "message", "format": "text"},
    ]
    urlparts = {
        "query": query,
        "widgetType": "list-view",
        "columns": json.dumps(columns),
        "newestAtBottom": "***",
        "showOnlyFirstLine": "false",
    }
    urlparts = apply_humio_time_filter(urlparts, time_filter)
    return urlunparse(
        ParseResult(
            "https", "cloud.us.humio.com", "/k8s/search", "", urlencode(urlparts), ""
        )
    )


def get_axiom_link_for_test_runner_logs(
    forge_namespace: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    """Get a link to the forge test runner logs in axiom for a given test run in a given namespace"""

    apl_query = f"""
        k8s
        | where ['k8s.cluster'] contains "forge" and ['k8s.container_name'] != "calico-node" and ['k8s.namespace'] != "calico-apiserver" and ['k8s.container_name'] != "kube-proxy" and 
        ['k8s.labels.app.kubernetes.io/name'] = "forge" and ['k8s.namespace'] == "{forge_namespace}"
        """

    logs_url = f"https://app.axiom.co/aptoslabs-hghf/explorer?initForm={urlquote(json.dumps({'apl': apl_query, 'queryOptions': apply_axiom_time_filter(time_filter), }))}"

    return logs_url


def get_axiom_link_for_node_logs(
    forge_namespace: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    """Get a link to the node logs in axiom for a given test run in a given namespace"""

    apl_query = f"""
        k8s
        | where ['k8s.cluster'] contains "forge" and ['k8s.container_name'] != "calico-node" and ['k8s.namespace'] != "calico-apiserver" and ['k8s.container_name'] != "kube-proxy" and 
            (
            ['k8s.namespace'] == "{forge_namespace}" // filters on namespace which contains validator logs
            or // remove either side of the OR operator to only display validator or forge-runner logs
            ['k8s.labels.forge-namespace'] == "{forge_namespace}" // filters on specific forge-runner pod in default namespace
            )
        """

    logs_url = f"https://app.axiom.co/aptoslabs-hghf/explorer?initForm={urlquote(json.dumps({'apl': apl_query, 'queryOptions': apply_axiom_time_filter(time_filter), }))}"

    return logs_url


def apply_axiom_time_filter(
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> Mapping:
    if time_filter is True:
        return {"quickRange": "30m"}
    elif isinstance(time_filter, tuple):
        return {
            "startTime": time_filter[0].astimezone(timezone.utc).isoformat(),
            "endTime": time_filter[1].astimezone(timezone.utc).isoformat(),
        }
    else:
        raise Exception(f"Invalid refresh argument: {time_filter}")


def format_github_info(context: ForgeContext) -> str:
    if not context.github_job_url:
        return ""
    else:
        return (
            textwrap.dedent(
                f"""
            * [Test runner output]({context.github_job_url})
            * Test run is {'' if context.forge_blocking else 'not '}land-blocking
            """
            )
            .lstrip()
            .strip()
        )


def get_testsuite_images(context: ForgeContext) -> str:
    # If image tags dont match then we're upgrading
    if context.image_tag != context.upgrade_image_tag:
        return f"`{context.image_tag}` ==> `{context.upgrade_image_tag}`"
    else:
        return f"`{context.image_tag}`"


def format_pre_comment(context: ForgeContext) -> str:
    dashboard_link = get_dashboard_link(
        context.forge_namespace,
        context.forge_chain_name,
        True,
    )
    humio_logs_link = get_humio_link_for_node_logs(
        context.forge_namespace,
        True,
    )
    axiom_logs_link = get_axiom_link_for_node_logs(
        context.forge_namespace,
        True,
    )
    validator_cpu_profile_link = get_cpu_profile_link(
        ContainerName.Validator,
        context.forge_namespace,
    )
    fullnode_cpu_profile_link = get_cpu_profile_link(
        ContainerName.FullNode,
        context.forge_namespace,
    )

    return (
        textwrap.dedent(
            f"""
            ### Forge is running suite `{context.forge_test_suite}` on {get_testsuite_images(context)}
            * [Grafana dashboard (auto-refresh)]({dashboard_link})
            * [Humio Logs]({humio_logs_link})
            * [Axiom Logs]({axiom_logs_link})
            * [Validator CPU Profile]({validator_cpu_profile_link})
            * [Fullnode CPU Profile]({fullnode_cpu_profile_link})
            """
        ).lstrip()
        + format_github_info(context)
    )


def format_comment(context: ForgeContext, result: ForgeResult) -> str:
    dashboard_link = get_dashboard_link(
        context.forge_namespace,
        context.forge_chain_name,
        (result.start_time, result.end_time),
    )
    humio_logs_link = get_humio_link_for_node_logs(
        context.forge_namespace,
        (result.start_time, result.end_time),
    )
    axiom_logs_link = get_axiom_link_for_node_logs(
        context.forge_namespace,
        (result.start_time, result.end_time),
    )
    validator_cpu_profile_link = get_cpu_profile_link(
        ContainerName.Validator,
        context.forge_namespace,
        result.start_time,
        result.end_time,
    )
    fullnode_cpu_profile_link = get_cpu_profile_link(
        ContainerName.FullNode,
        context.forge_namespace,
        result.start_time,
        result.end_time,
    )

    if result.state == ForgeState.PASS:
        forge_comment_header = f"### :white_check_mark: Forge suite `{context.forge_test_suite}` success on {get_testsuite_images(context)}"
    elif result.state == ForgeState.SOFT_FAIL:
        forge_comment_header = f"### :heavy_exclamation_mark: Forge suite `{context.forge_test_suite}` soft failure on {get_testsuite_images(context)}"
    elif result.state == ForgeState.FAIL:
        forge_comment_header = f"### :x: Forge suite `{context.forge_test_suite}` hard failure on {get_testsuite_images(context)}"
    elif result.state == ForgeState.SKIP:
        forge_comment_header = f"### :thought_balloon: Forge suite `{context.forge_test_suite}` preempted on {get_testsuite_images(context)}"
    else:
        raise Exception(f"Invalid forge state: {result.state}")

    return (
        textwrap.dedent(
            f"""
        {forge_comment_header}
        ```
        """
        ).lstrip()
        + format_report(context, result)
        + textwrap.dedent(
            f"""
        ```
        * [Grafana dashboard]({dashboard_link})
        * [Humio Logs]({humio_logs_link})
        * [Axiom Logs]({axiom_logs_link})
        * [Validator CPU Profile]({validator_cpu_profile_link})
        * [Fullnode CPU Profile]({fullnode_cpu_profile_link})
        """
        )
        + format_github_info(context)
    )


BEGIN_JUNIT = "=== BEGIN JUNIT ==="
END_JUNIT = "=== END JUNIT ==="


def format_junit_xml(_context: ForgeContext, result: ForgeResult) -> str:
    forge_output = result.output
    start_index = forge_output.find(BEGIN_JUNIT)
    if start_index == -1:
        raise Exception(
            "=== BEGIN JUNIT === not found in forge output, unable to write junit xml"
        )

    start_index += len(BEGIN_JUNIT)
    if start_index > len(forge_output):
        raise Exception(
            "=== BEGIN JUNIT === found at end of forge output, unable to write junit xml"
        )

    end_index = forge_output.find(END_JUNIT)
    if end_index == -1:
        raise Exception(
            "=== END JUNIT === not found in forge output, unable to write junit xml"
        )

    return forge_output[start_index:end_index].strip().lstrip()


class ForgeRunner:
    def run(self, context: ForgeContext) -> ForgeResult:
        raise NotImplementedError


def dump_forge_state(
    shell: Shell,
    forge_namespace: str,
    kubeconf: Optional[str] = None,
) -> str:
    try:
        assert kubeconf is not None, "kubeconf is required"
        output = (
            shell.run(
                [
                    "kubectl",
                    "--kubeconfig",
                    kubeconf,
                    "get",
                    "pods",
                    "-n",
                    forge_namespace,
                ]
            )
            .unwrap()
            .decode()
        )
        return "" if "No resources found" in output else output
    except Exception as e:
        return f"Failed to get debugging output: {e}"


def find_the_killer(
    shell: Shell,
    forge_namespace: str,
    kubeconf: str,
) -> str:
    killer = shell.run(
        [
            "kubectl",
            "--kubeconfig",
            kubeconf,
            "get",
            "pod",
            "-l",
            f"forge-namespace={forge_namespace}",
            "-o",
            "jsonpath={.items[0].metadata.name}",
        ]
    ).output.decode()
    return f"Likely killed by {killer}"


class LocalForgeRunner(ForgeRunner):
    def run(self, context: ForgeContext) -> ForgeResult:
        # Set rlimit to unlimited for txn emitter locally
        context.filesystem.rlimit(
            resource.RLIMIT_NOFILE, resource.RLIM_INFINITY, resource.RLIM_INFINITY
        )

        with ForgeResult.with_context(context) as forge_result:
            result = context.shell.run(
                context.forge_args,
                stream_output=True,
            )
            forge_result.set_output(result.output.decode())
            forge_result.set_state(
                ForgeState.PASS if result.succeeded() else ForgeState.FAIL
            )

        return forge_result


class K8sForgeRunner(ForgeRunner):
    def delete_forge_runner_pod(self, context: ForgeContext):
        log.info(f"Deleting forge pod for namespace {context.forge_namespace}")
        assert context.forge_cluster.kubeconf is not None, "kubeconf is required"
        context.shell.run(
            [
                "kubectl",
                "--kubeconfig",
                context.forge_cluster.kubeconf,
                *context.forge_cluster.kubectl_create_context_arg,
                "delete",
                "pod",
                "-n",
                "default",
                "-l",
                f"forge-namespace={context.forge_namespace}",
                "--force",
            ]
        )
        context.shell.run(
            [
                "kubectl",
                "--kubeconfig",
                context.forge_cluster.kubeconf,
                "wait",
                "-n",
                "default",
                "--for=delete",
                "pod",
                "-l",
                f"forge-namespace={context.forge_namespace}",
            ]
        )

    def run(self, context: ForgeContext) -> ForgeResult:
        forge_pod_name = sanitize_forge_resource_name(
            f"{context.forge_namespace}-{context.time.epoch()}-{context.image_tag}",
            max_length=52 if context.forge_cluster.is_multiregion else 63,
        )
        assert context.forge_cluster.kubeconf is not None, "kubeconf is required"

        self.delete_forge_runner_pod(context)

        if context.filesystem.exists(FORGE_TEST_RUNNER_TEMPLATE_PATH):
            template = context.filesystem.read(FORGE_TEST_RUNNER_TEMPLATE_PATH)
        else:
            template = context.filesystem.read(
                os.path.join("testsuite", FORGE_TEST_RUNNER_TEMPLATE_PATH)
            )
        forge_triggered_by = "github-actions" if context.github_actions else "other"

        assert context.aws_account_num is not None, "AWS account number is required"

        # determine the interal image repos based on the context of where the cluster is located
        if context.cloud == Cloud.AWS:
            forge_image_full = f"{context.aws_account_num}.dkr.ecr.{context.aws_region}.amazonaws.com/{ECR_REPO_PREFIX}/forge:{context.forge_image_tag}"
            validator_node_selector = "eks.amazonaws.com/nodegroup: validators"
        elif (
            context.cloud == Cloud.GCP
        ):  # the GCP project for images is separate than the cluster
            forge_image_full = f"{GAR_REPO_NAME}/forge:{context.forge_image_tag}"
            validator_node_selector = ""  # no selector
            # TODO: also no NAP node selector yet
            # TODO: also registries need to be set up such that the default compute service account can access it:  $PROJECT_ID-compute@developer.gserviceaccount.com
        else:
            raise Exception(f"Unknown cloud: {context.cloud}")

        rendered = template.decode().format(
            FORGE_POD_NAME=forge_pod_name,
            FORGE_IMAGE_TAG=context.forge_image_tag,
            IMAGE_TAG=context.image_tag,
            UPGRADE_IMAGE_TAG=context.upgrade_image_tag,
            FORGE_IMAGE=forge_image_full,
            FORGE_NAMESPACE=context.forge_namespace,
            FORGE_ARGS=" ".join(context.forge_args),
            FORGE_TRIGGERED_BY=forge_triggered_by,
            FORGE_TEST_SUITE=sanitize_k8s_resource_name(context.forge_test_suite),
            FORGE_USERNAME=sanitize_k8s_resource_name(context.forge_username),
            FORGE_RETAIN_DEBUG_LOGS=context.forge_retain_debug_logs,
            FORGE_JUNIT_XML_PATH=context.forge_junit_xml_path,
            VALIDATOR_NODE_SELECTOR=validator_node_selector,
            KUBECONFIG=MULTIREGION_KUBECONFIG_PATH,
            MULTIREGION_KUBECONFIG_DIR=MULTIREGION_KUBECONFIG_DIR,
        )

        log.info(f"rendered_forge_test_runner: {rendered}")

        with ForgeResult.with_context(context) as forge_result:
            specfile = context.filesystem.mkstemp()
            context.filesystem.write(specfile, rendered.encode())
            context.shell.run(
                [
                    "kubectl",
                    "--kubeconfig",
                    context.forge_cluster.kubeconf,
                    *context.forge_cluster.kubectl_create_context_arg,
                    "apply",
                    "-n",
                    "default",
                    "-f",
                    specfile,
                ]
            ).unwrap()
            context.shell.run(
                [
                    "kubectl",
                    "--kubeconfig",
                    context.forge_cluster.kubeconf,
                    "wait",
                    "-n",
                    "default",
                    "--timeout=5m",
                    "--for=condition=Ready",
                    f"pod/{forge_pod_name}",
                ]
            )
            state = None
            attempts = 100
            streaming = True
            while state is None:
                forge_logs = context.shell.run(
                    [
                        "kubectl",
                        "--kubeconfig",
                        context.forge_cluster.kubeconf,
                        "logs",
                        "-n",
                        "default",
                        "-f",
                        forge_pod_name,
                    ],
                    stream_output=streaming,
                )

                # After the first invocation, stop streaming duplicate logs
                if streaming:
                    streaming = False

                forge_result.set_output(forge_logs.output.decode())

                # parse the pod status: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
                forge_status = (
                    context.shell.run(
                        [
                            "kubectl",
                            "--kubeconfig",
                            context.forge_cluster.kubeconf,
                            "get",
                            "pod",
                            "-n",
                            "default",
                            forge_pod_name,
                            "-o",
                            "jsonpath='{.status.phase}'",
                        ]
                    )
                    .output.decode()
                    .lower()
                )

                if "running" in forge_status:
                    continue
                elif "succeeded" in forge_status:
                    state = ForgeState.PASS
                elif re.findall(r"not\s*found", forge_status, re.IGNORECASE):
                    state = ForgeState.SKIP
                    forge_result.set_debugging_output(
                        find_the_killer(
                            context.shell,
                            context.forge_namespace,
                            context.forge_cluster.kubeconf,
                        )
                    )
                else:
                    exit_code = context.shell.run(
                        [
                            "kubectl",
                            "--kubeconfig",
                            context.forge_cluster.kubeconf,
                            "get",
                            "pod",
                            "-n",
                            "default",
                            forge_pod_name,
                            "-o",
                            "jsonpath={.status.containerStatuses[*].state.terminated.exitCode}",
                        ]
                    ).output.decode()
                    log.info(f"Forge runner exit code: {exit_code}")
                    if exit_code == "51":
                        state = ForgeState.SOFT_FAIL
                    else:
                        state = ForgeState.FAIL

                attempts -= 1
                if attempts <= 0:
                    raise Exception("Exhausted attempt to get forge pod status")

            forge_result.set_state(state)

        # cleanup the pod manually
        self.delete_forge_runner_pod(context)

        return forge_result


def get_aws_account_num(shell: Shell) -> str:
    caller_id = shell.run(["aws", "sts", "get-caller-identity"])
    return json.loads(caller_id.unwrap()).get("Account")


# NOTE: this is not used anywhere
def get_current_cluster_name(shell: Shell) -> str:
    result = shell.run(["kubectl", "config", "current-context"])
    current_context = result.unwrap().decode()
    matches = re.findall(r"aptos.*", current_context)
    if len(matches) != 1:
        raise ValueError("Could not determine current cluster name: {current_context}")
    return matches[0]


def add_build_variant_prefix(image_tag: str, variant: str) -> str:
    """Add the necessary image tag prefix to specify the correct image tag for the build variant"""
    variant_prefix = BUILD_VARIANT_TAG_PREFIX_MAP[variant]
    if not image_tag.startswith(variant_prefix):
        return f"{variant_prefix}_{image_tag}"
    return image_tag


def ensure_provided_image_tags_has_profile_or_features(
    image_tag: Optional[str],
    upgrade_image_tag: Optional[str],
    enable_failpoints: bool,
    enable_performance_profile: bool,
) -> Tuple[str, str]:
    """
    Ensure that the build variant specified is reflected in the image tag. If not, then return the image tag
    with the prefix that is expected
    """
    ret = []
    for tag in [image_tag, upgrade_image_tag]:
        curr_tag = None
        if not tag:
            pass
        elif enable_failpoints:
            curr_tag = add_build_variant_prefix(tag, "failpoints")
        elif enable_performance_profile:
            curr_tag = add_build_variant_prefix(tag, "performance")
        else:
            curr_tag = tag
        ret.append(curr_tag)

    return tuple(ret)


def find_recent_images_by_profile_or_features(
    shell: Shell,
    git: Git,
    num_images: int,
    enable_failpoints: Optional[bool],
    enable_performance_profile: Optional[bool],
    cloud: Cloud = Cloud.GCP,
) -> Sequence[str]:
    image_tag_prefix = ""
    if enable_failpoints and enable_performance_profile:
        raise Exception(
            "Cannot yet set both testing (failpoints) image and performance"
        )

    if enable_performance_profile:
        image_tag_prefix = "performance_"
    if enable_failpoints:
        image_tag_prefix = "failpoints_"

    return find_recent_images(
        shell,
        git,
        num_images,
        image_name=VALIDATOR_TESTING_IMAGE_NAME,
        image_tag_prefixes=[image_tag_prefix],
        cloud=cloud,
    )


def find_recent_images(
    shell: Shell,
    git: Git,
    num_images: int,
    image_name: str,
    image_tag_prefixes: List[str] = [""],
    commit_threshold: int = 100,
    cloud: Cloud = Cloud.GCP,
) -> Sequence[str]:
    """
    Find the last `num_images` images built from the current git repo by searching the git commit history
    Also optionally filter by images with the provided prefixes, such as those denoting specific build variants
    (e.g. cargo profiles and feature flags enabled)
    """

    # implicitly add the empty prefix, which will get the default release build without a prefix
    if len(image_tag_prefixes) == 0:
        image_tag_prefixes.append("")

    # the number of images we need to find is actually the number of unique images
    # multiplied by the number of image tag prefixes (e.g. variants) we expect to find
    num_variants = len(image_tag_prefixes)
    num_images_with_variants = num_images * num_variants

    ret = []  # the list of images we will return
    for revision in git.last(commit_threshold):
        temp_ret = []  # count variants for this revision
        for prefix in image_tag_prefixes:
            image_tag = f"{prefix}{revision}"
            exists = image_exists(shell, image_name, image_tag, cloud=cloud)
            if exists:
                temp_ret.append(image_tag)
            if len(temp_ret) >= num_variants:
                ret.extend(temp_ret)
        if len(ret) >= num_images_with_variants:  # we have enough images
            break
    if len(ret) < num_images_with_variants:
        raise Exception(
            f"Could not find {num_images} recent images with prefixes {image_tag_prefixes}"
        )

    return ret


def image_exists(
    shell: Shell,
    image_name: str,
    image_tag: str,
    cloud: Cloud = Cloud.GCP,
) -> bool:
    """Check if an image exists in a given repository"""
    if cloud == Cloud.GCP:
        full_image = f"{GAR_REPO_NAME}/{image_name}:{image_tag}"
        return shell.run(
            [
                "crane",
                "manifest",
                full_image,
            ],
            stream_output=True,
        ).succeeded()
    elif cloud == Cloud.AWS:
        full_image = f"{ECR_REPO_PREFIX}/{image_name}:{image_tag}"
        log.info(f"Checking if image exists in GCP: {full_image}")
        return shell.run(
            [
                "aws",
                "ecr",
                "describe-images",
                "--repository-name",
                f"{ECR_REPO_PREFIX}/{image_name}",
                "--image-ids",
                f"imageTag={image_tag}",
            ],
            stream_output=True,
        ).succeeded()
    else:
        raise Exception(f"Unknown cloud repo type: {cloud}")


def sanitize_k8s_resource_name(resource: str, max_length: int = 63) -> str:
    sanitized_resource = ""
    for i, c in enumerate(resource):
        if i >= max_length:
            break
        if c.isalnum():
            sanitized_resource += c
        else:
            sanitized_resource += "-"  # Replace the invalid character with a '-'

    if sanitized_resource.endswith("-"):
        sanitized_resource = (
            sanitized_resource[:-1] + "0"
        )  # Set the last character to '0'

    return sanitized_resource


def sanitize_forge_resource_name(forge_resource: str, max_length: int = 63) -> str:
    """
    Sanitize the intended forge resource name to be a valid k8s resource name.
    Resource names must be: (i) 63 characters or less; (ii) contain characters
    that are alphanumeric, '-', or '.'; (iii) start and end with an alphanumeric
    character; and (iv) start with "forge-".
    """
    if not forge_resource.startswith("forge-"):
        raise Exception("Forge resource name must start with 'forge-'")

    return sanitize_k8s_resource_name(forge_resource, max_length=max_length)


def create_forge_command(
    forge_runner_mode: Optional[str],
    forge_test_suite: Optional[str],
    forge_runner_duration_secs: Optional[str],
    forge_num_validators: Optional[str],
    forge_num_validator_fullnodes: Optional[str],
    image_tag: str,
    upgrade_image_tag: str,
    forge_namespace: str,
    forge_namespace_reuse: Optional[str],
    forge_namespace_keep: Optional[str],
    forge_enable_haproxy: Optional[str],
    forge_enable_indexer: Optional[str],
    forge_deployer_profile: Optional[str],
    cargo_args: Optional[Sequence[str]],
    forge_cli_args: Optional[Sequence[str]],
    test_args: Optional[Sequence[str]],
) -> List[str]:
    """
    Cargo args get passed before forge directly to cargo (i.e. features)
    Forge Cli args get passed to forge before the test command (i.e. test suite)
    Test args get passed to the test subcommand (i.e. image tag)
    """
    if forge_runner_mode == "local":
        forge_args = [
            "cargo",
            "run",
        ]
        if cargo_args:
            forge_args.extend(cargo_args)
        forge_args.extend(
            [
                "-p",
                "aptos-forge-cli",
                "--",
            ]
        )
    elif forge_runner_mode == "k8s":
        forge_args = ["forge"]
    else:
        return []
    if forge_test_suite:
        forge_args.extend(["--suite", forge_test_suite])
    if forge_runner_duration_secs:
        forge_args.extend(["--duration-secs", forge_runner_duration_secs])

    if forge_num_validators:
        forge_args.extend(["--num-validators", forge_num_validators])
    if forge_num_validator_fullnodes:
        forge_args.extend(
            [
                "--num-validator-fullnodes",
                forge_num_validator_fullnodes,
            ]
        )

    if forge_cli_args:
        forge_args.extend(forge_cli_args)

    # TODO: add support for other backend
    backend = "k8s-swarm"
    forge_args.extend(
        [
            "test",
            backend,
            "--image-tag",
            image_tag,
            "--upgrade-image-tag",
            upgrade_image_tag,
            "--namespace",
            forge_namespace,
        ]
    )

    if forge_runner_mode == "local":
        forge_args.append("--port-forward")

    if forge_namespace_reuse == "true":
        forge_args.append("--reuse")
    if forge_namespace_keep == "true":
        forge_args.append("--keep")
    if forge_enable_haproxy == "true":
        forge_args.append("--enable-haproxy")
    if forge_enable_indexer == "true":
        forge_args.append("--enable-indexer")
    if forge_deployer_profile:
        forge_args.extend(["--deployer-profile", forge_deployer_profile])

    if test_args:
        forge_args.extend(test_args)

    return forge_args


async def run_multiple(
    context: SystemContext,
    forge_test_suites: List[str],
    disabled_suites: Set[str],
    forge_namespace: str,
    forge_pre_comment: Optional[str],
    forge_comment: Optional[str],
    forge_runner_mode: Optional[str],
    github_step_summary: Optional[str],
) -> None:
    # Remove formatting environment variables
    os.environ["FORGE_OUTPUT"] = ""
    os.environ["FORGE_REPORT"] = ""
    os.environ["FORGE_PRE_COMMENT"] = ""
    os.environ["FORGE_COMMENT"] = ""
    os.environ["GITHUB_STEP_SUMMARY"] = ""

    pending_results = []
    pending_suites = []
    pending_comment = []

    start_time = context.time.now()

    for suite in forge_test_suites:
        new_namespace = f"{forge_namespace}-{suite}"
        humio_link = get_humio_link_for_test_runner_logs(new_namespace, True)
        axiom_link = get_axiom_link_for_test_runner_logs(new_namespace, True)
        pending_comment.append(f"Running {suite}: [Runner logs in Humio]({humio_link})")
        pending_comment.append(f"Running {suite}: [Runner logs in Axiom]({axiom_link})")
        if forge_runner_mode != "pre-forge":
            pending_results.append(
                context.shell.gen_run(
                    [
                        # TODO figure out which other args we should forward
                        # This might only work from github for starters
                        sys.executable,
                        __file__,
                        "test",
                        "--forge-test-suite",
                        suite,
                        "--forge-namespace",
                        new_namespace,
                    ],
                )
            )
            pending_suites.append((suite, new_namespace))
    log.info("\n".join(pending_comment))
    if forge_runner_mode == "pre-forge":
        if forge_pre_comment:
            context.filesystem.write(
                forge_pre_comment,
                "\n".join(pending_comment).encode(),
            )
    else:
        final_forge_comment = []
        results = await asyncio.gather(*pending_results)
        stop_time = context.time.now()
        assert len(results) == len(pending_suites)
        failed = False
        for i, result in enumerate(results):
            suite, namespace = pending_suites[i]
            if result.succeeded():
                final_forge_comment.append(f"{suite} succeeded")
            else:
                failed = suite not in disabled_suites
                disabled = " (disabled)" if suite in disabled_suites else ""
                final_forge_comment.append(f"{suite} failed{disabled}")
        final_forge_comment.append(f"Run {'failed' if failed else 'succeeded'}")
        log.info("\n".join(final_forge_comment))
        if forge_comment:
            context.filesystem.write(
                forge_comment, "\n".join(final_forge_comment).encode()
            )
        if github_step_summary:
            context.filesystem.write(
                github_step_summary, "\n".join(final_forge_comment).encode()
            )


def seeded_random_choice(namespace: str, cluster_names: Sequence[str]) -> str:
    random.seed(namespace)
    return random.choice(cluster_names)


@main.command()
# output files
@envoption("FORGE_OUTPUT")
@envoption("FORGE_REPORT")
@envoption("FORGE_PRE_COMMENT")
@envoption("FORGE_COMMENT")
@envoption("GITHUB_STEP_SUMMARY")
# cluster auth
# FIXME: Remove (deprecated).
@envoption("CLOUD")
@envoption("AWS_REGION", "us-west-2")
@envoption("GCP_ZONE", "us-central1-c")
# forge test runner customization
@envoption("FORGE_RUNNER_MODE", "k8s")
@envoption("FORGE_CLUSTER_NAME")
# these override the args in forge-cli
@envoption("FORGE_NUM_VALIDATORS")
@envoption("FORGE_NUM_VALIDATOR_FULLNODES")
@envoption("FORGE_NAMESPACE_KEEP")
@envoption("FORGE_NAMESPACE_REUSE")
@envoption("FORGE_ENABLE_HAPROXY")
@envoption("FORGE_ENABLE_INDEXER")
@envoption("FORGE_DEPLOYER_PROFILE")
@envoption("FORGE_ENABLE_FAILPOINTS")
@envoption("FORGE_ENABLE_PERFORMANCE")
@envoption("FORGE_RUNNER_DURATION_SECS", "300")
@envoption("FORGE_IMAGE_TAG")
@envoption("FORGE_RETAIN_DEBUG_LOGS", "false")
@envoption("FORGE_JUNIT_XML_PATH")
@envoption("FORGE_TEST_SUITE")
@envoption("IMAGE_TAG")
@envoption("UPGRADE_IMAGE_TAG")
@envoption("FORGE_NAMESPACE")
@envoption("VERBOSE")
@envoption("GITHUB_ACTIONS", "false")
@click.option("--balance-clusters", is_flag=True)
@envoption("FORGE_BLOCKING", "true")
@envoption("GITHUB_SERVER_URL")
@envoption("GITHUB_REPOSITORY")
@envoption("GITHUB_RUN_ID")
@click.option(
    "--cargo-args",
    multiple=True,
    help="Cargo args to pass to forge local runner",
)
@click.option(
    "--forge-cli-args", multiple=True, help="Forge cli args to pass to forge cli"
)
@click.option(
    "--test-args", multiple=True, help="Test args to pass to forge test subcommand"
)
@click.argument("test_suites", nargs=-1)
def test(
    forge_output: Optional[str],
    forge_report: Optional[str],
    forge_pre_comment: Optional[str],
    forge_comment: Optional[str],
    cloud: str,
    aws_region: str,
    gcp_zone: str,
    forge_runner_mode: str,
    forge_cluster_name: Optional[str],
    forge_num_validators: Optional[str],
    forge_num_validator_fullnodes: Optional[str],
    forge_namespace_keep: Optional[str],
    forge_namespace_reuse: Optional[str],
    forge_enable_failpoints: Optional[str],
    forge_enable_performance: Optional[str],
    forge_enable_haproxy: Optional[str],
    forge_enable_indexer: Optional[str],
    forge_deployer_profile: Optional[str],
    forge_test_suite: str,
    forge_runner_duration_secs: str,
    forge_image_tag: Optional[str],
    forge_retain_debug_logs: str,
    forge_junit_xml_path: Optional[str],
    image_tag: Optional[str],
    upgrade_image_tag: Optional[str],
    forge_namespace: Optional[str],
    verbose: Optional[str],
    github_actions: str,
    balance_clusters: bool,
    forge_blocking: Optional[str],
    github_server_url: Optional[str],
    github_repository: Optional[str],
    github_run_id: Optional[str],
    github_step_summary: Optional[str],
    cargo_args: Optional[List[str]],
    forge_cli_args: Optional[List[str]],
    test_args: Optional[List[str]],
    test_suites: Tuple[str],
) -> None:
    """Run a forge test"""

    if verbose:
        log.setLevel(logging.DEBUG)

    ### XXX: hack these arguments to force Forge to run with overrides
    # forge_cluster_name = "aptos-forge-0"
    # forge_enable_performance = "true"

    log.debug("Initializing backends...")

    # Initialize all configs
    shell = LocalShell()
    git = Git(shell)
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.init()

    log.debug("Finished sourcing configs")

    # XXX: manual override testing in CI
    # forge_cluster_name = "aptos-forge-0"

    # # for performance
    # forge_enable_performance = "true"

    if not forge_namespace:
        forge_namespace = f"forge-{processes.user()}-{time.epoch()}"

    assert forge_namespace is not None, "Forge namespace is required"
    assert len(forge_namespace) <= 63, "Forge namespace must be 63 characters or less"

    forge_namespace = sanitize_forge_resource_name(forge_namespace)

    all_suites = list(test_suites)
    if forge_test_suite:
        all_suites.append(forge_test_suite)
    if not all_suites:
        forge_test_suite = "default"

    # Resolve suites to tests
    all_resolved_suites = []
    enabled_resolved_suites = []
    for suite in all_suites:
        config_suites = config.get("test_suites")
        if suite in config_suites:
            enabled_resolved_suites.extend(config_suites[suite]["enabled_tests"].keys())
            all_resolved_suites.extend(config_suites[suite]["all_tests"].keys())
        else:
            enabled_resolved_suites.append(suite)
            all_resolved_suites.append(suite)

    disabled_resolved_suites = set(all_resolved_suites) - set(enabled_resolved_suites)

    if len(all_resolved_suites) == 0:
        log.info("No tests to run")
        return
    elif len(all_resolved_suites) == 1:
        forge_test_suite = enabled_resolved_suites[0]
    else:
        asyncio.run(
            run_multiple(
                context,
                all_resolved_suites,
                disabled_resolved_suites,
                forge_namespace,
                forge_pre_comment,
                forge_comment,
                forge_runner_mode,
                github_step_summary,
            )
        )
        return

    aws_account_num = None
    try:
        aws_account_num = get_aws_account_num(shell)
    except Exception as e:
        log.warning(f"Warning: failed to get AWS account number: {e}")

    # Perform cluster selection
    if not forge_cluster_name or balance_clusters:
        cluster_names = config.get("enabled_clusters")
        forge_cluster_name = seeded_random_choice(forge_namespace, cluster_names)

    assert forge_cluster_name, "Forge cluster name is required"

    # cloud
    if cloud is not None:
        log.warning(
            "Explicitly setting the cloud is deprecated. The cloud is now inferred from the cluster name."
        )
    if "big" in forge_cluster_name:
        cloud_enum = Cloud.AWS
    else:
        cloud_enum = Cloud.GCP

    if forge_cluster_name == "forge-multiregion":
        log.info("Using multiregion cluster")
        forge_cluster = ForgeCluster(
            name=forge_cluster_name,
            cloud=Cloud.GCP,
            region="multiregion",
            kubeconf=context.filesystem.mkstemp(),
            is_multiregion=True,
        )
    else:
        log.info(
            f"Looking for cluster {forge_cluster_name} in cloud {cloud_enum.value}"
        )
        forge_cluster = find_forge_cluster(
            context.shell, cloud_enum, forge_cluster_name, context.filesystem.mkstemp()
        )
        log.info(f"Found cluster: {forge_cluster}")

    asyncio.run(forge_cluster.write(context.shell))

    # These features and profile flags are set as strings
    enable_failpoints = forge_enable_failpoints == "true"
    enable_performance_profile = forge_enable_performance == "true"

    # In the below, assume that the image is pushed to all registries
    # across all clouds and supported regions
    image_tag, upgrade_image_tag = ensure_provided_image_tags_has_profile_or_features(
        image_tag,
        upgrade_image_tag,
        enable_failpoints=enable_failpoints,
        enable_performance_profile=enable_performance_profile,
    )

    if forge_test_suite == "compat":
        # Compat uses 2 image tags
        default_latest_image, second_latest_image = list(
            find_recent_images_by_profile_or_features(
                shell,
                git,
                2,
                enable_failpoints=enable_failpoints,
                enable_performance_profile=enable_performance_profile,
                cloud=cloud_enum,
            )
        )
        # This might not work as intended because we dont know if that revision
        # passed forge
        image_tag = image_tag or second_latest_image
        forge_image_tag = forge_image_tag or default_latest_image
        upgrade_image_tag = upgrade_image_tag or default_latest_image
    else:
        # All other tests use just one image tag
        # Only try finding exactly 1 image
        default_latest_image = find_recent_images_by_profile_or_features(
            shell,
            git,
            1,
            enable_failpoints=enable_failpoints,
            enable_performance_profile=enable_performance_profile,
            cloud=cloud_enum,
        )[0]

        image_tag = image_tag or default_latest_image
        forge_image_tag = forge_image_tag or default_latest_image
        upgrade_image_tag = upgrade_image_tag or default_latest_image

    image_tag, upgrade_image_tag = ensure_provided_image_tags_has_profile_or_features(
        image_tag,
        upgrade_image_tag,
        enable_failpoints=enable_failpoints,
        enable_performance_profile=enable_performance_profile,
    )

    assert image_tag is not None, "Image tag is required"
    assert forge_image_tag is not None, "Forge image tag is required"
    assert upgrade_image_tag is not None, "Upgrade image tag is required"

    log.info("Using the following image tags:")
    log.info(f"\tforge:  {forge_image_tag}")
    log.info(f"\tswarm:  {image_tag}")
    log.info(f"\tswarm upgrade (if applicable):  {upgrade_image_tag}")

    # finally, whether we've derived the image tags or used the user-inputted ones, check if they exist
    assert image_exists(
        shell, VALIDATOR_TESTING_IMAGE_NAME, image_tag, cloud=cloud_enum
    ), f"swarm (validator) image does not exist: {image_tag}"
    assert image_exists(
        shell, VALIDATOR_TESTING_IMAGE_NAME, upgrade_image_tag, cloud=cloud_enum
    ), f"swarm upgrade (validator) image does not exist: {upgrade_image_tag}"
    assert image_exists(
        shell, FORGE_IMAGE_NAME, forge_image_tag, cloud=cloud_enum
    ), f"forge (test runner) image does not exist: {forge_image_tag}"

    forge_args = create_forge_command(
        forge_runner_mode=forge_runner_mode,
        forge_test_suite=forge_test_suite,
        forge_runner_duration_secs=forge_runner_duration_secs,
        forge_num_validators=forge_num_validators,
        forge_num_validator_fullnodes=forge_num_validator_fullnodes,
        image_tag=image_tag,
        upgrade_image_tag=upgrade_image_tag,
        forge_namespace=forge_namespace,
        forge_namespace_reuse=forge_namespace_reuse,
        forge_namespace_keep=forge_namespace_keep,
        forge_enable_haproxy=forge_enable_haproxy,
        forge_enable_indexer=forge_enable_indexer,
        forge_deployer_profile=forge_deployer_profile,
        cargo_args=cargo_args,
        forge_cli_args=forge_cli_args,
        test_args=test_args,
    )

    log.info("forge_args: %s", forge_args)

    # use the github actor username if possible
    forge_username = os.getenv("GITHUB_ACTOR") or "unknown-username"
    forge_context = ForgeContext(
        shell=shell,
        filesystem=filesystem,
        processes=processes,
        time=time,
        # cluster auth
        cloud=cloud_enum,
        aws_account_num=aws_account_num,
        aws_region=aws_region,
        gcp_zone=gcp_zone,
        forge_image_tag=forge_image_tag,
        image_tag=image_tag,
        upgrade_image_tag=upgrade_image_tag,
        forge_namespace=forge_namespace,
        forge_cluster=forge_cluster,
        forge_test_suite=forge_test_suite,
        forge_username=forge_username,
        forge_retain_debug_logs=forge_retain_debug_logs,
        forge_junit_xml_path=forge_junit_xml_path,
        forge_blocking=forge_blocking == "true",
        github_actions=github_actions,
        github_job_url=(
            f"{github_server_url}/{github_repository}/actions/runs/{github_run_id}"
            if github_run_id
            else None
        ),
        forge_args=forge_args,
    )
    forge_runner_mapping = {
        "local": LocalForgeRunner,
        "k8s": K8sForgeRunner,
    }

    # Maybe this should be its own command?
    pre_comment = format_pre_comment(forge_context)
    if forge_pre_comment:
        forge_context.report(
            ForgeResult.empty(),
            [ForgeFormatter(forge_pre_comment, lambda *_: pre_comment)],
        )
    else:
        log.info(pre_comment)

    if forge_runner_mode == "pre-forge":
        return

    try:
        forge_runner = forge_runner_mapping[forge_runner_mode]()
        result = forge_runner.run(forge_context)

        outputs = []
        if forge_output:
            outputs.append(ForgeFormatter(forge_output, lambda *_: result.output))
        if forge_report:
            outputs.append(ForgeFormatter(forge_report, format_report))
        else:
            log.info(format_report(forge_context, result))
        if forge_comment:
            outputs.append(ForgeFormatter(forge_comment, format_comment))
        else:
            log.info(format_comment(forge_context, result))
        if github_step_summary:
            outputs.append(ForgeFormatter(github_step_summary, format_comment))
        if forge_junit_xml_path:
            outputs.append(ForgeFormatter(forge_junit_xml_path, format_junit_xml))

        forge_context.report(result, outputs)

        log.info(result.format(forge_context))

        if not result.succeeded() and forge_blocking == "true":
            raise SystemExit(1)

    except Exception as e:
        raise Exception(
            "\n".join(
                [
                    "Forge state:",
                    dump_forge_state(
                        shell,
                        forge_namespace,
                        forge_cluster.kubeconf,
                    ),
                ]
            )
        ) from e


async def get_all_forge_jobs(
    context: SystemContext,
    clusters: List[str],
) -> List[ForgeJob]:
    # Get all cluster contexts
    all_jobs = []
    tempfiles = []
    for cluster in clusters:
        temp = context.filesystem.mkstemp()
        config = ForgeCluster(name=cluster, kubeconf=temp)
        try:
            await config.write(context.shell)
            tempfiles.append(temp)
            all_jobs.extend(await config.get_jobs(context.shell))
        except Exception as e:
            log.info(f"Failed to get jobs from cluster: {cluster}: {e}")

    def unlink_tempfiles():
        for temp in tempfiles:
            context.filesystem.unlink(temp)

    # Delay the deletion of cluster files till the process terminates
    context.processes.atexit(unlink_tempfiles)

    return all_jobs


@main.group("job")
def job() -> None:
    """Subcommands for managing forge jobs"""
    pass


@job.command("list")
@click.option("--phase", multiple=True, help="Only show jobs in this phase")
@click.option("--regex", help="Only show jobs matching this regex")
def list_jobs(
    phase: List[str],
    regex: str,
) -> None:
    """List all running forge jobs"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.init()

    # Default to show running jobs
    phase = phase or ["Running"]

    pattern = re.compile(regex or ".*")
    jobs = asyncio.run(get_all_forge_jobs(context, config.get("all_clusters")))

    for job in jobs:
        if not pattern.match(job.name) or phase and job.phase not in phase:
            continue
        if job.succeeded():
            fg = "green"
        elif job.failed():
            fg = "red"
        elif job.running():
            fg = "yellow"
        else:
            fg = "white"

        click.secho(
            f"{job.cluster.name} {job.name} {job.phase}: (num_fullnodes: {job.num_fullnodes}, num_validators: {job.num_validators})",
            fg=fg,
        )


@main.command()
@click.argument("job_name")
def tail(
    job_name: str,
) -> None:
    """Tail the logs for a running job"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.init()

    job_name = sanitize_forge_resource_name(job_name)

    all_jobs = asyncio.run(get_all_forge_jobs(context, config.get("all_clusters")))
    found_jobs = [job for job in all_jobs if job.name == job_name]
    if not found_jobs:
        other_jobs = "".join(
            ["\n\t- " + job.name for job in all_jobs if job.phase == "Running"]
        )
        raise Exception(f"Couldnt find job {job_name}, instead found {other_jobs}")
    elif len(found_jobs) > 1:
        raise Exception(f"Found multiple jobs for name {job_name}")
    job = found_jobs[0]
    assert job.cluster.kubeconf is not None, "kubeconf is required"
    shell.run(
        [
            "kubectl",
            "logs",
            "--kubeconfig",
            job.cluster.kubeconf,
            "-n",
            "default",
            "-f",
            job_name,
        ],
        stream_output=True,
    ).unwrap()


class TestConfig(TypedDict):
    name: str


class TestSuite(TypedDict):
    name: str
    all_tests: Mapping[str, TestConfig]
    enabled_tests: Mapping[str, TestConfig]


# All changes to this struct must be backwards compatible
# i.e. its ok to add a new field, but not to remove one
class ForgeConfigValue(TypedDict):
    enabled_clusters: List[str]
    all_clusters: List[str]
    test_suites: Mapping[str, TestSuite]
    default_helm_values: Mapping


def default_forge_config() -> ForgeConfigValue:
    # Return a default config with not all the fields, as they are not mandatory
    # This ensures we check for backwards compatibility
    return {  # type: ignore
        "enabled_clusters": [],
        "all_clusters": [],
    }


def validate_forge_config_default_helm_values(value: Any) -> List[str]:
    """Validate that the given forge config has a valid default_helm_values config. Returns a list of error messages"""
    errors = []
    try:
        keys = value["default_helm_values"].keys()
        for chart in HELM_CHARTS:
            if chart not in keys:
                errors.append(f"Missing required chart {chart} in default_helm_values")
    except Exception as e:
        errors.append(f"Invalid default_helm_values: {e}")

    return errors


def validate_forge_config(value: Any) -> List[str]:
    """Validate that the given forge config has all the required fields. Returns a list of error messages"""
    errors = []
    if not isinstance(value, dict):
        return ["Value must be derived from dict"]

    for field in default_forge_config().keys():
        if field not in value:
            errors.append(f"Missing required field {field}")
    if errors:
        return errors
    for cluster in value["enabled_clusters"]:
        if not isinstance(cluster, str):
            errors.append("Cluster must be a string")

    return errors


def ensure_forge_config(value: Any) -> ForgeConfigValue:
    # TODO: find a better way to do this
    errors = validate_forge_config(value)
    if errors:
        raise Exception("Type had errors:\n" + "\n".join(errors))
    return value


def get_forge_config_diff(
    old_config: ForgeConfigValue,
    new_config: ForgeConfigValue,
    full_diff: Optional[bool] = False,
) -> Iterator[str]:
    """Returns a list of diffs between the old and new config"""
    config_string = json.dumps(new_config, indent=2)
    old_config_string = json.dumps(old_config, indent=2)
    old_lines = old_config_string.splitlines()
    new_lines = config_string.splitlines()
    if full_diff:
        diff = difflib.Differ()
        return diff.compare(old_lines, new_lines)
    else:
        return difflib.unified_diff(old_lines, new_lines)


class ForgeConfigBackend:
    def create(self) -> None:
        raise NotImplementedError()

    def write(self, config: object) -> None:
        raise NotImplementedError()

    def read(self) -> object:
        raise NotImplementedError()


@dataclass
class S3ForgeConfigBackend(ForgeConfigBackend):
    system: SystemContext
    name: str
    key: str = DEFAULT_CONFIG_KEY

    def create(self) -> None:
        self.system.shell.run(["aws", "s3", "mb", f"s3://{self.name}"]).unwrap()

    def write(self, config: object) -> None:
        temp = self.system.filesystem.mkstemp()
        self.system.filesystem.write(temp, json.dumps(config).encode())
        self.system.shell.run(
            [
                "aws",
                "s3api",
                "put-object",
                "--bucket",
                self.name,
                "--key",
                self.key,
                "--body",
                temp,
            ]
        ).unwrap()

    def read(self) -> object:
        temp = self.system.filesystem.mkstemp()
        self.system.shell.run(
            [
                "aws",
                "s3api",
                "get-object",
                "--bucket",
                self.name,
                "--key",
                self.key,
                temp,
            ]
        ).unwrap()
        return json.loads(self.system.filesystem.read(temp))


@dataclass
class FilesystemConfigBackend(ForgeConfigBackend):
    filename: str
    system: SystemContext

    def create(self) -> None:
        # We dont need to do anything special
        pass

    def write(self, config: object) -> None:
        self.system.filesystem.write(
            self.filename,
            json.dumps(config).encode(),
        )

    def read(self) -> object:
        return json.loads(self.system.filesystem.read(self.filename))


class ForgeConfig:
    NONE_SENTINEL = object()

    def __init__(self, backend: ForgeConfigBackend) -> None:
        self.backend = backend
        self.config: ForgeConfigValue = default_forge_config()

    def create(self) -> None:
        self.backend.create()

    def init(self) -> None:
        self.config = ensure_forge_config(self.backend.read())

    def get(self, key: str, default: Optional[Any] = NONE_SENTINEL) -> Any:
        value = self.config.get(key, default)
        if value is self.NONE_SENTINEL:
            raise Exception(f"Missing key {key} in Forge config")
        return value

    def set(self, key, value, validate: bool = True) -> None:
        new_config = {**self.config, key: value}
        if validate:
            self.config = ensure_forge_config(new_config)
        else:
            self.config = new_config  # type: ignore

    def flush(self) -> None:
        self.backend.write(self.config)

    def dump(self) -> ForgeConfigValue:
        return ForgeConfigValue(**self.config)


@main.group()
def config() -> None:
    """Manage forge configuration"""
    pass


@config.command("create")
def create_config() -> None:
    """Create a new forge config at the default location"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.create()


@config.command("get")
@click.argument("key", type=str, required=False)
def get_config(key: Optional[str]) -> None:
    """Print the forge configuration"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.init()

    try:
        val = config.dump().get(key) if key else config.dump()
        if not val:
            raise Exception(f"No config value found for key {key}")
    except Exception as e:
        raise click.ClickException(str(e))

    # print the config as JSON so it looks nicer
    config_string = json.dumps(val, indent=2)
    log.info(config_string)


def keyword_argument(value: str) -> Tuple[str, str]:
    if "=" not in value:
        raise click.BadParameter("Must be in the form KEY=VALUE")
    key, value = value.split("=", 1)
    return (key, eval(value))


@config.command("set")
@click.option("--force", is_flag=True, help="Disable config validation")
@click.option(
    "--config", "config_path", help="Provide a file to replace the current config"
)
@click.argument("values", type=keyword_argument, nargs=-1)
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def set_config(
    force: Optional[bool],
    config_path: Optional[str],
    values: List[Tuple[str, str]],
    y: bool,
) -> None:
    """Replace forge configuration values with local config"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    old_config = deepcopy(config.dump())

    if config_path:
        local_config = ForgeConfig(FilesystemConfigBackend(config_path, context))
        local_config.init()
        for k, v in local_config.dump().items():
            config.set(k, v, validate=not force)
    else:
        # If we dont have a local file, read from config
        config.init()

    for k, v in values:
        config.set(k, v, validate=not force)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@config.command("edit")
@click.pass_context
def config_edit(ctx: click.Context) -> None:
    """Edit forge configuration via interactive text editor"""
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    context = SystemContext(shell, filesystem, processes, SystemTime())
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))
    config.init()

    temp = filesystem.mkstemp()
    filesystem.write(temp, json.dumps(config.dump(), indent=2).encode())
    editor = os.getenv("EDITOR", "vim")
    os.system(f"{editor} {temp}")
    ctx.invoke(set_config, config_path=temp)


@config.group("helm")
def helm_config() -> None:
    """Manage forge helm configuration"""
    pass


def assert_helm_chart_valid(chart: str) -> None:
    if chart not in HELM_CHARTS:
        raise Exception(f"Invalid helm chart {chart}")


@helm_config.command("get")
@click.argument("chart", type=str, required=True)
def helm_config_get(chart: str) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    assert_helm_chart_valid(chart)

    config.init()

    default_helm_values = config.get("default_helm_values").get(chart)
    if not default_helm_values:
        raise Exception(f"No helm values found for chart {chart}")
    log.info(json.dumps(default_helm_values, indent=2))


@helm_config.command("set")
@click.argument("chart", type=str, required=True)
@click.option(
    "--config",
    "config_path",
    help="Provide a file to replace the current config",
    required=True,
)
@click.option("--force", is_flag=True, help="Disable config validation")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def helm_config_set(
    chart: str, config_path: str, force: Optional[bool], y: bool
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    assert_helm_chart_valid(chart)

    # read existing config
    config.init()

    # read new helm config and set it as a corresponding key
    local_config = FilesystemConfigBackend(config_path, context)
    local_config_values = local_config.read()
    old_config = deepcopy(config.dump())

    # set the default_helm_values key if it doesnt exist
    try:
        config.get("default_helm_values")
    except Exception:
        config.set("default_helm_values", {}, validate=not force)

    # merge the local configs into the existing config
    new_default_helm_values = {
        **config.get("default_helm_values"),
        **{chart: local_config_values},
    }

    config.set("default_helm_values", new_default_helm_values, validate=not force)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@config.group("cluster")
def cluster_config() -> None:
    """Manage forge cluster configuration"""
    pass


@cluster_config.command("delete")
@click.argument("cluster")
@click.option("--force", is_flag=True, help="Disable config validation")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def cluster_config_delete(
    cluster: str,
    force: Optional[bool],
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    context = SystemContext(shell, filesystem, processes, SystemTime())
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    enabled_clusters = config.get("enabled_clusters")
    if cluster in enabled_clusters and not force:
        raise Exception(f"Cluster {cluster} is enabled, use --force to delete anyway")
    all_clusters = config.get("all_clusters")

    try:
        all_clusters.remove(cluster)
    except ValueError:
        raise Exception(f"Cluster {cluster} does not exist")

    config.set("all_clusters", all_clusters)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@cluster_config.command("add")
@click.argument("cluster")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def cluster_config_add(cluster: str, y: bool) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    all_clusters = config.get("all_clusters")
    if cluster in all_clusters:
        raise Exception(f"Cluster {cluster} already exists")
    all_clusters.append(cluster)
    config.set("all_clusters", all_clusters)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@cluster_config.command("enable")
@click.argument("cluster")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def cluster_config_enable(cluster: str, y: bool) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    enabled_clusters = config.get("enabled_clusters")
    if cluster in enabled_clusters:
        raise Exception(f"Cluster {cluster} is already enabled")
    enabled_clusters.append(cluster)
    config.set("enabled_clusters", enabled_clusters)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@cluster_config.command("disable")
@click.argument("cluster")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def cluster_config_disable(
    cluster: str,
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    enabled_clusters = config.get("enabled_clusters")
    enabled_clusters.remove(cluster)
    config.set("enabled_clusters", enabled_clusters)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@cluster_config.command("list")
def cluster_config_list() -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()

    enabled_clusters = config.get("enabled_clusters")
    for cluster in config.get("all_clusters"):
        if cluster in enabled_clusters:
            fg = "green"
            enabled = " [enabled]"
        else:
            fg = "white"
            enabled = ""

        click.secho(f"{cluster}{enabled}", fg=fg)

    config.flush()


@config.group("test")
def test_config() -> None:
    """Manage forge test configuration"""
    pass


@test_config.command("add")
@click.argument("suite_name")
@click.argument("test_name", required=False)
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def test_config_add(
    suite_name: str,
    test_name: Optional[str],
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    suites = config.get("test_suites")
    if suites is None:
        raise Exception("Failed to get suites")
    test_suite = suites.get(
        suite_name,
        {
            "name": suite_name,
            "all_tests": {},
            "enabled_tests": {},
        },
    )

    if test_name in test_suite["all_tests"]:
        raise Exception(f"Test {test_name} already exists")

    if test_name:
        test_suite["all_tests"][test_name] = {
            "name": test_name,
        }

    suites[suite_name] = test_suite

    config.set("test_suites", suites)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@test_config.command("show")
@click.argument("suite", required=False)
def test_config_show(
    suite: Optional[str],
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()

    test_suites = config.get("test_suites")
    for suite_name in test_suites:
        log.info(f"suite: {suite_name}")
        if suite and suite_name != suite:
            continue
        suite_config = test_suites[suite_name]
        for test_name in suite_config["all_tests"]:
            if test_name in suite_config.get("enabled_tests"):
                fg = "green"
                enabled = " [enabled]"
            else:
                fg = ""
                enabled = ""

            click.secho(f" - {test_name}{enabled}", fg=fg)


@test_config.command("list")
def test_config_list() -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()

    test_suites = config.get("test_suites")

    for suite_name in test_suites:
        log.info(f"suite: {suite_name}")


@test_config.command("delete")
@click.argument("suite_name")
@click.argument("test_name", required=False)
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def test_config_delete(
    suite_name: str,
    test_name: Optional[str],
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    suites = config.get("test_suites")

    if test_name:
        suite_config = suites.get(suite_name)
        if test_name in suite_config.get("enabled_tests"):
            raise Exception(f"Cannot delete enabled test {test_name}")
        del suite_config["enabled_tests"][test_name]
        suites[suite_name] = suite_config
    else:
        del suites[suite_name]

    config.set("test_suites", suites)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@test_config.command("enable")
@click.argument("suite_name")
@click.argument("test_name")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def test_config_enable(
    suite_name: str,
    test_name: str,
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    suites = config.get("test_suites")
    suite_config = suites.get(suite_name)

    if test_name in suite_config.get("enabled_tests"):
        raise Exception(f"{test_name} is already enabled")

    test_config = suite_config.get("all_tests").get(test_name)
    if test_config is None:
        raise Exception(f"Cannot find test {test_name}")

    suite_config["enabled_tests"][test_name] = test_config
    suites[suite_name] = suite_config

    config.set("test_suites", suites)

    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


@test_config.command("disable")
@click.argument("suite_name")
@click.argument("test_name")
@click.option("-y", is_flag=True, help="Accept all interactive prompts")
def test_config_disable(
    suite_name: str,
    test_name: str,
    y: bool,
) -> None:
    shell = LocalShell()
    filesystem = LocalFilesystem()
    processes = SystemProcesses()
    time = SystemTime()
    context = SystemContext(shell, filesystem, processes, time)
    config = ForgeConfig(S3ForgeConfigBackend(context, DEFAULT_CONFIG))

    config.init()
    old_config = deepcopy(config.dump())

    suites = config.get("test_suites")
    suite_config = suites.get(suite_name)

    if test_name not in suite_config.get("enabled_tests"):
        raise Exception(f"{test_name} is not enabled")

    test_config = suite_config.get("all_tests").get(test_name)
    if test_config is None:
        raise Exception(f"Cannot find test {test_name}")

    del suite_config["enabled_tests"][test_name]
    suites[suite_name] = suite_config

    config.set("test_suites", suites)
    d = get_forge_config_diff(old_config, config.dump())
    log.info("\n".join(d))
    if y or get_prompt_answer("Would you like to apply the config change now?"):
        config.flush()
    else:
        log.info("Config not updated")


if __name__ == "__main__":
    main()
