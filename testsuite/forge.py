from __future__ import annotations

import json
import multiprocessing


# Using fork can crash the subprocess, use spawn instead
multiprocessing.set_start_method('spawn')


from optparse import Option
import os
import pwd
import random
import re
import resource
import subprocess
import sys
import tempfile
import textwrap
import time
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Callable, Dict, Generator, List, Optional, Sequence, Tuple, TypedDict, Union


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


class Shell:
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        raise NotImplementedError


@dataclass
class LocalShell(Shell):
    verbose: bool = False

    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        # Write to a temp file, stream to stdout
        tmpname = tempfile.mkstemp()[1]
        with open(tmpname, 'wb') as writer, open(tmpname, 'rb') as reader:
            if self.verbose:
                print(f"+ {' '.join(command)}")
            process = subprocess.Popen(command, stdout=writer, stderr=writer)
            output = b""
            while process.poll() is None:
                chunk = reader.read()
                output += chunk
                if stream_output:
                    sys.stdout.write(chunk.decode("utf-8"))
                time.sleep(0.1)
            output += reader.read()
        return RunResult(process.returncode, output)


class FakeShell(Shell):
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        return RunResult(0, b'output')


def install_dependency(dependency: str) -> None:
    print(f"{dependency} is not currently installed")
    answer = os.getenv("FORGE_INSTALL_DEPENDENCIES") or os.getenv("CI")
    if not answer:
        answer = input("Would you like to install it now? (y/n) ").strip().lower()
    if answer in ("y", "yes", "yeet", "yessir", "si", "true"):
        shell = LocalShell(True)
        shell.run(["pip3", "install", dependency], stream_output=True).unwrap()
    else:
        print(f"Please install click (pip install {dependency}) and try again")
        exit(1)


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


def get_current_user() -> str:
    return pwd.getpwuid(os.getuid())[0]


def get_utc_timestamp(dt: datetime) -> str:
    return dt.strftime("%Y-%m-%dT%H:%M:%S.000Z")


@click.group()
def main() -> None:
    # Check that the current directory is the root of the repository.
    if not os.path.exists('.git'):
        print('This script must be run from the root of the repository.')
        raise SystemExit(1)


def envoption(name: str, default: Optional[Any] = None) -> Any:
    return click.option(
        f"--{name.lower().replace('_', '-')}",
        default=lambda: os.getenv(name, default() if callable(default) else default),
        show_default=True,
    )


class Filesystem:
    def write(self, filename: str, contents: bytes) -> None:
        raise NotImplementedError()

    def read(self, filename: str) -> bytes:
        raise NotImplementedError()

    def mkstemp(self) -> str:
        raise NotImplementedError()

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        raise NotImplementedError()


class FakeFilesystem(Filesystem):
    def write(self, filename: str, contents: bytes) -> None:
        print(f"Wrote {contents} to {filename}")

    def read(self, filename: str) -> bytes:
        return b"fake"

    def mkstemp(self) -> str:
        return "temp"

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        return


class LocalFilesystem(Filesystem):
    def write(self, filename: str, contents: bytes) -> None:
        with open(filename, 'wb') as f:
            f.write(contents)

    def read(self, filename: str) -> bytes:
        with open(filename, 'rb') as f:
            return f.read()

    def mkstemp(self) -> str:
        return tempfile.mkstemp()[1]

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        resource.setrlimit(resource_type, (soft, hard))

# o11y resources
INTERN_ES_DEFAULT_INDEX = "90037930-aafc-11ec-acce-2d961187411f"
INTERN_ES_BASE_URL = "https://es.intern.aptosdev.com"
INTERN_GRAFANA_BASE_URL = (
    "https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&"
    "var-Datasource=Remote%20Prometheus%20Intern"
)
DEVINFRA_ES_DEFAULT_INDEX = "d0bc5e20-badc-11ec-9a50-89b84ac337af"
DEVINFRA_ES_BASE_URL = "https://es.devinfra.aptosdev.com"
DEVINFRA_GRAFANA_BASE_URL = (
    "https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&"
    "var-Datasource=Remote%20Prometheus%20Devinfra"
)
HUMIO_LOGS_LINK = (
    "https://cloud.us.humio.com/k8s/search?query=%24forgeLogs%28validator_insta"
    "nce%3Dvalidator-0%29%20%7C%20$FORGE_NAMESPACE%20&live=true&start=24h&widge"
    "tType=list-view&columns=%5B%7B%22type%22%3A%22field%22%2C%22fieldName%22%3"
    "A%22%40timestamp%22%2C%22format%22%3A%22timestamp%22%2C%22width%22%3A180%7"
    "D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22level%22%2C%22forma"
    "t%22%3A%22text%22%2C%22width%22%3A54%7D%2C%7B%22type%22%3A%22link%22%2C%22"
    "openInNewBrowserTab%22%3Atrue%2C%22style%22%3A%22button%22%2C%22hrefTempla"
    "te%22%3A%22https%3A%2F%2Fgithub.com%2Faptos-labs%2Faptos-core%2Fpull%2F%7B"
    "%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22textTemplate%22%3A%22%7"
    "B%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22header%22%3A%22Forge%2"
    "0PR%22%2C%22width%22%3A79%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%"
    "22%3A%22k8s.namespace%22%2C%22format%22%3A%22text%22%2C%22width%22%3A104%7"
    "D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.pod_name%22%2C%"
    "22format%22%3A%22text%22%2C%22width%22%3A126%7D%2C%7B%22type%22%3A%22field"
    "%22%2C%22fieldName%22%3A%22k8s.container_name%22%2C%22format%22%3A%22text%"
    "22%2C%22width%22%3A85%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3"
    "A%22message%22%2C%22format%22%3A%22text%22%7D%5D&newestAtBottom=true&showO"
    "nlyFirstLine=false"
)


def prometheus_port_forward() -> None:
    os.execvp("kubectl", ["kubectl", "port-forward", "prometheus", "9090"])


class Process:
    def name(self) -> str:
        raise NotImplementedError()
    
    def kill(self) -> None:
        raise NotImplementedError()

    def ppid(self) -> int:
        raise NotImplementedError()


@dataclass
class FakeProcess(Process):
    _name: str
    _ppid: int

    def name(self) -> str:
        return self._name

    def kill(self) -> None:
        print(f"killing {self._name}")

    def ppid(self) -> int:
        return self._ppid


class Processes:
    def processes(self) -> Generator[Process, None, None]:
        raise NotImplementedError()

    def get_pid(self) -> int:
        raise NotImplementedError()

    def spawn(self, target: Callable[[], None]) -> Process:
        raise NotImplementedError()


@dataclass
class SystemProcess(Process):
    process: psutil.Process

    def name(self) -> str:
        return self.process.name()

    def kill(self) -> None:
        self.process.kill()


@dataclass
class MultiProcessingProcess(Process):
    process: multiprocessing.Process

    def name(self) -> str:
        return self.process.name

    def ppid(self) -> int:
        # Since we spawn this process for all intents and purposes we are its
        # parent process
        return os.getpid()

    def kill(self) -> None:
        self.process.terminate()
        self.process.join()


class SystemProcesses(Processes):
    def processes(self) -> Generator[Process, None, None]:
        for process in psutil.process_iter():
            yield SystemProcess(process)

    def get_pid(self) -> int:
        return os.getpid()

    def spawn(self, target: Callable[[], None]) -> Process:
        process = multiprocessing.Process(daemon=True, target=target)
        process.start()
        return MultiProcessingProcess(process)


class FakeProcesses(Processes):
    def processes(self) -> Generator[Process, None, None]:
        yield FakeProcess("concensus", 1)

    def get_pid(self) -> int:
        return 2

    def spawn(self, target: Callable[[], None]) -> Process:
        return FakeProcess("child", 2)


class ForgeState(Enum):
    RUNNING = "RUNNING"
    PASS = "PASS"
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

    @classmethod
    def from_args(cls, state: ForgeState, output: str) -> "ForgeResult":
        result = cls()
        result.state = state
        result.output = output
        return result

    @classmethod
    def empty(cls) -> "ForgeResult":
        return cls.from_args(ForgeState.EMPTY, "")

    @classmethod
    @contextmanager
    def with_context(cls, context: "ForgeContext") -> Generator["ForgeResult", None, None]:
        result = cls()
        result.state = ForgeState.RUNNING
        result._start_time = context.time.now()
        try:
            yield result
        except Exception as e:
            result.set_state(ForgeState.FAIL)
            result.set_debugging_output(
                "Error: {}\nDebugging Output:{}\n".format(
                    str(e),
                    dump_forge_state(context.shell, context.forge_namespace)
                )
            )
        result._end_time = context.time.now()
        if result.state not in (ForgeState.PASS, ForgeState.FAIL, ForgeState.SKIP):
            raise Exception("Forge result never entered terminal state")
        if result.output is None:
            raise Exception("Forge result didnt record output")

    def set_state(self, state: ForgeState) -> None:
        self.state = state

    def set_output(self, output: str) -> None:
        self.output = output

    def set_debugging_output(self, output: str) -> None:
        self.debugging_output = output

    def format(self) -> str:
        return f"Forge {self.state.value.lower()}ed"

    def succeeded(self) -> bool:
        return self.state == ForgeState.PASS


class Time:
    def epoch(self) -> str:
        return self.now().strftime('%s')

    def now(self) -> datetime:
        raise NotImplementedError()


class SystemTime(Time):
    def now(self) -> datetime:
        return datetime.now(timezone.utc)


class FakeTime(Time):
    _now: datetime = datetime.fromisoformat("2022-07-29T00:00:00+00:00")

    def now(self) -> datetime:
        return self._now


@dataclass
class ForgeContext:
    shell: Shell
    filesystem: Filesystem
    processes: Processes
    time: Time

    # forge criteria
    forge_test_suite: str
    local_p99_latency_ms_threshold: str
    forge_runner_tps_threshold: str
    forge_runner_duration_secs: str
    
    # forge cluster options
    forge_namespace: str
    reuse_args: Sequence[str]
    keep_args: Sequence[str]
    haproxy_args: Sequence[str]

    # aws related options
    aws_account_num: str
    aws_region: str

    forge_image_tag: str
    image_tag: str
    upgrade_image_tag: str
    forge_namespace: str
    forge_cluster_name: str
    forge_blocking: bool

    github_actions: str
    github_job_url: Optional[str]

    def report(self, result: ForgeResult, outputs: List[ForgeFormatter]) -> None:
        for formatter in outputs:
            output = formatter.format(self, result)
            print(f"=== Start {formatter} ===")
            print(output)
            print(f"=== End {formatter} ===")
            self.filesystem.write(formatter.filename, output.encode())

    @property
    def forge_chain_name(self) -> str:
        forge_chain_name = self.forge_namespace.lstrip("aptos-")
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
        error_output,
        result.debugging_output
    )
    if not report_lines:
        return "Forge test runner terminated:\n{}".format(debugging_appendix)
    report_text = None
    try:
        report_text = json.loads(report_output).get("text")
    except Exception as e:
        return "Forge report malformed: {}\n{}\n{}".format(e, report_output, debugging_appendix)
    if not report_text:
        return "Forge report text empty. See test runner output.\n{}".format(debugging_appendix)
    else:
        if result.state == ForgeState.FAIL:
            return "{}\n{}".format(report_text, debugging_appendix)
        return report_text


def get_validator_logs_link(
    forge_namespace: str,
    forge_chain_name: str,
    time_filter: Union[bool, Tuple[datetime, datetime]],
) -> str:
    es_base_url = DEVINFRA_ES_BASE_URL if "forge" in forge_chain_name else INTERN_ES_BASE_URL
    es_default_index = DEVINFRA_ES_DEFAULT_INDEX if "forge" in forge_chain_name else INTERN_ES_DEFAULT_INDEX
    val0_hostname = "aptos-node-0-validator-0"

    if time_filter is True:
        es_time_filter = "refreshInterval:(pause:!f,value:10000),time:(from:now-15m,to:now)"
    elif isinstance(time_filter, tuple):
        es_start_time = time_filter[0].strftime("%Y-%m-%dT%H:%M:%S.000Z")
        es_end_time = time_filter[1].strftime("%Y-%m-%dT%H:%M:%S.000Z")
        es_time_filter = f"refreshInterval:(pause:!t,value:0),time:(from:'{es_start_time}',to:'{es_end_time}')"
    else:
        raise Exception(f"Invalid refresh argument: {time_filter}")

    return f"""
        {es_base_url}/_dashboards/app/discover#/?
        _g=(filters:!(), {es_time_filter})
        &_a=(
            columns:!(_source),
            filters:!((
                '$state':(store:appState),
                meta:(
                    alias:!n,
                    disabled:!f,
                    index:'{es_default_index}',
                    key:chain_name,
                    negate:!f,
                    params:(query:{forge_chain_name}),
                    type:phrase
                ),
                query:(match_phrase:(chain_name:{forge_chain_name}))
            ),
            (
                '$state':(store:appState),
                meta:(
                    alias:!n,
                    disabled:!f,
                    index:'{es_default_index}',
                    key:namespace,
                    negate:!f,
                    params:(query:{forge_namespace}),
                    type:phrase
                ),
                query:(match_phrase:(namespace:{forge_namespace}))
            ),
            (
                '$state':(store:appState),
                meta:(
                    alias:!n,
                    disabled:!f,
                    index:'{es_default_index}',
                    key:hostname,
                    negate:!f,
                    params:(query:{val0_hostname}),
                    type:phrase),
                    query:(match_phrase:(hostname:{val0_hostname})
                )
            )),
            index:'{es_default_index}',
            interval:auto,query:(language:kuery,query:''),sort:!()
        )
    """.replace(" ", "").replace("\n", "")


def get_dashboard_link(
    forge_cluster_name: str,
    forge_namespace: str,
    forge_chain_name: str,
    time_filter: Union[bool, Tuple[datetime, datetime]]
) -> str:
    if time_filter is True:
        grafana_time_filter = "&refresh=10s&from=now-15m&to=now"
    elif isinstance(time_filter, tuple):
        milliseconds = lambda dt: int(dt.strftime("%f")) / 1000
        start_ms = milliseconds(time_filter[0])
        end_ms = milliseconds(time_filter[1])
        grafana_time_filter = f"&from={start_ms}&to={end_ms}"
    else:
        raise Exception(f"Invalid refresh argument: {time_filter}")

    base_url = DEVINFRA_GRAFANA_BASE_URL if "forge" in forge_cluster_name else INTERN_GRAFANA_BASE_URL
    return f"{base_url}&var-namespace={forge_namespace}&var-chain_name={forge_chain_name}{grafana_time_filter}"



def get_humio_logs_link(forge_namespace: str) -> str:
    return HUMIO_LOGS_LINK.replace("$FORGE_NAMESPACE", forge_namespace)


def format_github_info(context: ForgeContext) -> str:
    return textwrap.dedent(
        f"""
          * [Test runner output]({context.github_job_url})
          * Test run is {'' if context.forge_blocking else 'not '}land-blocking
        """
    )


def format_pre_comment(context: ForgeContext) -> str:
    dashboard_link = "https://banana"
    validator_logs_link = get_validator_logs_link(context.forge_namespace, context.forge_chain_name, True)
    humio_logs_link = get_humio_logs_link(context.forge_namespace)

    return textwrap.dedent(
        f"""
        ### Forge is running with `{context.image_tag}`
        * [Grafana dashboard (auto-refresh)]({dashboard_link})
        * [Validator 0 logs (auto-refresh)]({validator_logs_link})
        * [Humio Logs]({humio_logs_link})
        """
    ).strip() + format_github_info(context)


def format_comment(context: ForgeContext, result: ForgeResult) -> str:
    dashboard_link = get_dashboard_link(
        context.forge_cluster_name,
        context.forge_namespace,
        context.forge_chain_name,
        (result.start_time, result.end_time),
    )
    validator_logs_link = get_validator_logs_link(
        context.forge_namespace,
        context.forge_chain_name,
        (result.start_time, result.end_time),
    )
    humio_logs_link = get_humio_logs_link(context.forge_namespace)

    if result.state == ForgeState.PASS:
        forge_comment_header = f"### :white_check_mark: Forge test success on `{context.image_tag}`"
    elif result.state == ForgeState.FAIL:
        forge_comment_header = f"### :x: Forge test perf regression on `{context.image_tag}`"
    elif result.state == ForgeState.SKIP:
        forge_comment_header = f"### :thought_balloon: Forge test preempted on `{context.image_tag}`"
    else:
        raise Exception(f"Invalid forge state: {result.state}")

    return textwrap.dedent(
        f"""
        {forge_comment_header}
        ```
        """
    ) + format_report(context, result) + textwrap.dedent(
        f"""
        ```
        * [Grafana dashboard (auto-refresh)]({dashboard_link})
        * [Validator 0 logs (auto-refresh)]({validator_logs_link})
        * [Humio Logs]({humio_logs_link})

        {result.format()}
        """
    ) + format_github_info(context)


class ForgeRunner:
    def run(self, context: ForgeContext) -> ForgeResult:
        raise NotImplementedError


def dump_forge_state(shell: Shell, forge_namespace: str) -> str:
    try:
        return shell.run([
            "kubectl", "get", "pods", "-n", forge_namespace,
        ]).unwrap().decode()
    except Exception as e:
        return f"Failed to get debugging output: {e}"


def find_the_killer(shell: Shell, forge_namespace) -> str:
    killer = shell.run([
        "kubectl", "get", "pod",
        "-l", f"forge-namespace={forge_namespace}",
        "-o", "jsonpath={.items[0].metadata.name}",
    ]).output.decode()
    return f"Likely killed by {killer}"


class LocalForgeRunner(ForgeRunner):
    def run(self, context: ForgeContext) -> ForgeResult:
        # Set rlimit to unlimited for txn emitter locally
        context.filesystem.rlimit(resource.RLIMIT_NOFILE, resource.RLIM_INFINITY, resource.RLIM_INFINITY)
        port_forward_process = context.processes.spawn(prometheus_port_forward)
        with ForgeResult.with_context(context) as forge_result:
            result = context.shell.run([
                "cargo", "run", "-p", "forge-cli",
                "--",
                "--suite", context.forge_test_suite,
                "--mempool-backlog", "5000",
                "--avg-tps", context.forge_runner_tps_threshold,
                "--max-latency-ms", context.local_p99_latency_ms_threshold,
                "--duration-secs", context.forge_runner_duration_secs,
                "test", "k8s-swarm",
                "--image-tag", context.image_tag,
                "--upgrade-image-tag", context.upgrade_image_tag,
                "--namespace", context.forge_namespace,
                "--port-forward",
                *context.reuse_args,
                *context.keep_args,
                *context.haproxy_args,
            ], stream_output=True)
            forge_result.set_output(result.output.decode())
            forge_result.set_state(ForgeState.PASS if result.succeeded() else ForgeState.FAIL)

        # Kill port forward unless we're keeping them
        if not context.keep_args:
            # Kill all processess with kubectl in the name
            for process in context.processes.processes():
                if 'kubectl' in process.name() and process.ppid() == context.processes.get_pid():
                    print("Killing", process)
                    process.kill()
            port_forward_process.kill()

        return forge_result


class K8sForgeRunner(ForgeRunner):
    def run(self, context: ForgeContext) -> ForgeResult:
        forge_pod_name = f"{context.forge_namespace}-{context.time.epoch()}-{context.image_tag}"[:64]
        context.shell.run([
            "kubectl", "delete", "pod",
            "-n", "default",
            "-l", f"forge-namespace={context.forge_namespace}",
            "--force"
        ])
        context.shell.run([
            "kubectl", "wait",
            "-n", "default",
            "--for=delete", "pod",
            "-l", f"forge-namespace={context.forge_namespace}",
        ])
        template = context.filesystem.read("testsuite/forge-test-runner-template.yaml")
        forge_triggered_by = "github-actions" if context.github_actions else "other"
        rendered = template.decode().format(
            FORGE_POD_NAME=forge_pod_name,
            FORGE_TEST_SUITE=context.forge_test_suite,
            FORGE_RUNNER_DURATION_SECS=context.forge_runner_duration_secs,
            FORGE_RUNNER_TPS_THRESHOLD=context.forge_runner_tps_threshold,
            FORGE_IMAGE_TAG=context.forge_image_tag,
            IMAGE_TAG=context.image_tag,
            UPGRADE_IMAGE_TAG=context.upgrade_image_tag,
            AWS_ACCOUNT_NUM=context.aws_account_num,
            AWS_REGION=context.aws_region,
            FORGE_NAMESPACE=context.forge_namespace,
            REUSE_ARGS=context.reuse_args if context.reuse_args else "",
            KEEP_ARGS=context.keep_args if context.keep_args else "",
            ENABLE_HAPROXY_ARGS=context.haproxy_args if context.haproxy_args else "",
            FORGE_TRIGGERED_BY=forge_triggered_by,
        )

        with ForgeResult.with_context(context) as forge_result:
            specfile = context.filesystem.mkstemp()
            context.filesystem.write(specfile, rendered.encode())
            context.shell.run([
                "kubectl", "apply", "-n", "default", "-f", specfile
            ]).unwrap()
            context.shell.run([
                "kubectl", "wait", "-n", "default", "--timeout=5m", "--for=condition=Ready", f"pod/{forge_pod_name}"
            ]).unwrap()
            forge_logs = context.shell.run([
                "kubectl", "logs", "-n", "default", "-f", forge_pod_name
            ], stream_output=True)

            forge_result.set_output(forge_logs.output.decode())

            state = None
            attempts = 100
            while state is None:
                # parse the pod status: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
                forge_status = context.shell.run([
                    "kubectl", "get", "pod", "-n", "default", forge_pod_name, "-o", "jsonpath='{.status.phase}'"
                ]).output.decode().lower()

                if "running" in forge_status:
                    continue
                elif "succeeded" in forge_status:
                    state = ForgeState.PASS
                elif re.findall(r"not\s*found", forge_status, re.IGNORECASE):
                    state = ForgeState.SKIP
                    forge_result.set_debugging_output(find_the_killer(context.shell, context.forge_namespace))
                else:
                    state = ForgeState.FAIL

                attempts -= 1
                if attempts <= 0:
                    raise Exception("Exhausted attempt to get forge pod status")

            forge_result.set_state(state)

        return forge_result


class AwsError(Exception):
    pass


def assert_aws_token_expiration(aws_token_expiration: Optional[str]) -> None:
    if aws_token_expiration is None:
        raise AwsError("AWS token is required")
    try:
        expiration = datetime.strptime(aws_token_expiration, "%Y-%m-%dT%H:%M:%S%z")
    except Exception as e:
        raise AwsError(f"Invalid date format: {aws_token_expiration}") from e
    if datetime.now(timezone.utc) > expiration:
        raise AwsError("AWS token has expired")


def get_aws_account_num(shell: Shell) -> str:
    caller_id = shell.run(["aws", "sts", "get-caller-identity"])
    return json.loads(caller_id.unwrap()).get("Account")


def assert_aws_auth(shell: Shell) -> None:
    # Simple read command which should fail
    list_eks_clusters(shell)


class ListClusterResult(TypedDict):
    clusters: List[str]


def list_eks_clusters(shell: Shell) -> List[str]:
    cluster_json = shell.run(["aws", "eks", "list-clusters"]).unwrap()
    # This type annotation is not enforced, just helpful
    try:
        cluster_result: ListClusterResult = json.loads(cluster_json)
        return [
            cluster_name
            for cluster_name in cluster_result["clusters"]
            if cluster_name.startswith("aptos-forge-")
        ]
    except Exception as e:
        raise AwsError("Failed to list eks clusters") from e


def set_current_cluster(shell: Shell, forge_cluster_name: str) -> None:
    shell.run(["aws", "eks", "update-kubeconfig", "--name", forge_cluster_name]).unwrap()


def update_aws_auth(shell: Shell, aws_auth_script: Optional[str] = None) -> None:
    if aws_auth_script is None:
        raise AwsError("Please authenticate with AWS and rerun")
    result = shell.run(["bash", "-c", f"source {aws_auth_script} && env | grep AWS_"])
    for line in result.unwrap().decode().splitlines():
        if line.startswith("AWS_"):
            key, val = line.split("=", 1)
            os.environ[key] = val
    assert_aws_auth(shell)


def get_current_cluster_name(shell: Shell) -> str:
    result = shell.run(["kubectl", "config", "current-context"])
    current_context = result.unwrap().decode()
    matches = re.findall(r"aptos.*", current_context)
    if len(matches) != 1:
        raise ValueError("Could not determine current cluster name: {current_context}")
    return matches[0]


@dataclass
class Git:
    shell: Shell

    def run(self, command) -> RunResult:
        return self.shell.run(["git", *command])

    def last(self, limit: int = 1) -> Generator[str, None, None]:
        for i in range(limit):
            yield self.run(["rev-parse", f"HEAD~{i}"]).unwrap().decode().strip()


def find_recent_images(
    shell: Shell,
    git: Git,
    num_images: int,
    # Set a generoush threshold in case of failures
    commit_threshold: int = 100,
) -> Generator[str, None, None]:
    i = 0
    j = 0
    for revision in git.last(commit_threshold):
        exists = image_exists(shell, revision)
        if exists:
            i += 1
            yield revision
        if i >= num_images:
            break
    if i < num_images:
        raise Exception(f"Could not find {num_images} recent images")


def image_exists(shell: Shell, image_tag: str) -> bool:
    result = shell.run([
        "aws", "ecr", "describe-images",
        "--repository-name", "aptos/validator",
        "--image-ids", f"imageTag={image_tag}"
    ])
    return result.exit_code == 0


@main.command()
# for calculating regression in local mode
@envoption("LOCAL_P99_LATENCY_MS_THRESHOLD", "60000")
# output files
@envoption("FORGE_OUTPUT")
@envoption("FORGE_REPORT")
@envoption("FORGE_PRE_COMMENT")
@envoption("FORGE_COMMENT")
# cluster auth
@envoption("AWS_REGION", "us-west-2")
@envoption("AWS_TOKEN_EXPIRATION")
@envoption("AWS_AUTH_SCRIPT")
# forge test runner customization
@envoption("FORGE_RUNNER_MODE", "k8s")
@envoption("FORGE_CLUSTER_NAME")
@envoption("FORGE_NAMESPACE_KEEP")
@envoption("FORGE_NAMESPACE_REUSE")
@envoption("FORGE_ENABLE_HAPROXY")
@envoption("FORGE_TEST_SUITE", "land_blocking")
@envoption("FORGE_RUNNER_DURATION_SECS", "300")
@envoption("FORGE_RUNNER_TPS_THRESHOLD", "400")
@envoption("FORGE_IMAGE_TAG")
@envoption("IMAGE_TAG")
@envoption("UPGRADE_IMAGE_TAG")
@envoption("FORGE_NAMESPACE")
@envoption("VERBOSE")
@envoption("GITHUB_ACTIONS", "false")
@click.option("--dry-run", is_flag=True)
@click.option("--ignore-cluster-warning", is_flag=True)
@click.option("--interactive/--no-interactive", is_flag=True, default=sys.stdin.isatty())
@click.option("--balance-clusters", is_flag=True)
@envoption("FORGE_BLOCKING", "true")
@envoption("GITHUB_SERVER_URL")
@envoption("GITHUB_REPOSITORY")
@envoption("GITHUB_RUN_ID")
@envoption("GITHUB_STEP_SUMMARY")
def test(
    local_p99_latency_ms_threshold: str,
    forge_output: Optional[str],
    forge_report: Optional[str],
    forge_pre_comment: Optional[str],
    forge_comment: Optional[str],
    aws_region: str,
    aws_token_expiration: Optional[str],
    aws_auth_script: Optional[str],
    forge_runner_mode: str,
    forge_cluster_name: Optional[str],
    forge_namespace_keep: Optional[str],
    forge_namespace_reuse: Optional[str],
    forge_enable_haproxy: Optional[str],
    forge_test_suite: str,
    forge_runner_duration_secs: str,
    forge_runner_tps_threshold: str,
    forge_image_tag: Optional[str],
    image_tag: Optional[str],
    upgrade_image_tag: Optional[str],
    forge_namespace: Optional[str],
    verbose: Optional[str],
    github_actions: str,
    dry_run: Optional[bool],
    ignore_cluster_warning: Optional[bool],
    interactive: bool,
    balance_clusters: bool,
    forge_blocking: Optional[str],
    github_server_url: Optional[str],
    github_repository: Optional[str],
    github_run_id: Optional[str],
    github_step_summary: Optional[str],
) -> None:
    shell = FakeShell() if dry_run else LocalShell(verbose == "true")
    git = Git(shell)
    filesystem = LocalFilesystem()
    processes = FakeProcesses() if dry_run else SystemProcesses()
    time = FakeTime() if dry_run else SystemTime()

    if dry_run:
        aws_account_num = "1234"
    # Pre flight checks
    else:
        try:
            assert_aws_auth(shell)
            aws_account_num = get_aws_account_num(shell)
        except Exception:
            update_aws_auth(shell, aws_auth_script)
            aws_account_num = get_aws_account_num(shell)

    if aws_auth_script and aws_token_expiration and not dry_run:
        assert_aws_token_expiration(os.getenv("AWS_TOKEN_EXPIRATION", aws_token_expiration))

    assert aws_account_num is not None, "AWS account number is required"

    # Perform cluster selection
    current_cluster = None
    if not forge_cluster_name:
        if interactive:
            current_cluster = get_current_cluster_name(shell)
            if click.confirm(f"Automatically using current cluster {current_cluster}"):
                forge_cluster_name = current_cluster

    if not forge_cluster_name or balance_clusters:
        cluster_names = list_eks_clusters(shell)
        forge_cluster_name = random.choice(cluster_names)

    assert forge_cluster_name, "Forge cluster name is required"

    click.echo(f"Using forge cluster: {forge_cluster_name}")
    if "forge" not in forge_cluster_name and not ignore_cluster_warning:
        click.echo("Forge cluster usually contains forge, to ignore this warning set --ignore-cluster-warning")
        if interactive:
            click.confirm("Continue?", abort=True)
        else:
            return

    set_current_cluster(shell, forge_cluster_name)

    if forge_namespace is None:
        forge_namespace = f"forge-{get_current_user()}-{time.epoch()}"

    assert forge_namespace is not None, "Forge namespace is required"

    default_latest_image, second_latest_image = list(find_recent_images(shell, git, 2))
    if forge_test_suite == "compat":
        # This might not work as intended because we dont know if that revision passed forge
        image_tag = image_tag or second_latest_image
        forge_image_tag = forge_image_tag or default_latest_image
        upgrade_image_tag = upgrade_image_tag or default_latest_image
    else:
        image_tag = image_tag or default_latest_image
        forge_image_tag = forge_image_tag or default_latest_image
        upgrade_image_tag = upgrade_image_tag or default_latest_image

    assert image_tag is not None, "Image tag is required"
    assert forge_image_tag is not None, "Forge image tag is required"
    assert upgrade_image_tag is not None, "Upgrade image tag is required"

    context = ForgeContext(
        shell=shell,
        filesystem=filesystem,
        processes=processes,
        time=time,

        forge_test_suite=forge_test_suite,
        local_p99_latency_ms_threshold=local_p99_latency_ms_threshold,
        forge_runner_tps_threshold=forge_runner_tps_threshold,
        forge_runner_duration_secs=forge_runner_duration_secs,

        reuse_args=["--reuse"] if forge_namespace_reuse else [],
        keep_args=["--keep"] if forge_namespace_keep else [],
        haproxy_args=["--enable-haproxy"] if forge_enable_haproxy else [],

        aws_account_num=aws_account_num,
        aws_region=aws_region,

        forge_image_tag=forge_image_tag,
        image_tag=image_tag,
        upgrade_image_tag=upgrade_image_tag,
        forge_namespace=forge_namespace,
        forge_cluster_name=forge_cluster_name,
        forge_blocking=forge_blocking == "true",

        github_actions=github_actions,
        github_job_url=f"{github_server_url}/{github_repository}/actions/runs/{github_run_id}",
    )
    forge_runner_mapping = {
        'local': LocalForgeRunner,
        'k8s': K8sForgeRunner,
    }

    # Maybe this should be its own command?
    pre_comment = format_pre_comment(context)
    if forge_pre_comment:
        context.report(
            ForgeResult.empty(),
            [ForgeFormatter(forge_pre_comment, lambda *_: pre_comment)],
        )

    if forge_runner_mode == 'pre-forge':
        return

    try:
        forge_runner = forge_runner_mapping[forge_runner_mode]()
        result = forge_runner.run(context)
        
        print(result.format())
        if not result.succeeded():
            print(result.debugging_output)

        outputs = []
        if forge_output:
            outputs.append(ForgeFormatter(forge_output, lambda *_: result.output))
        if forge_report:
            outputs.append(ForgeFormatter(forge_report, format_report))
        if forge_comment:
            outputs.append(ForgeFormatter(forge_comment, format_comment))
        if github_step_summary:
            outputs.append(ForgeFormatter(github_step_summary, format_comment))
        context.report(result, outputs)

        if not result.succeeded() and forge_blocking == "true":
            raise SystemExit(1)

    except Exception as e:
        raise Exception("Forge state:\n" + dump_forge_state(shell, forge_namespace)) from e


@dataclass
class ForgeJob:
    name: str
    phase: str

    @classmethod
    def from_pod(cls, pod: Dict[str, Any]) -> ForgeJob:
        return cls(name=pod["metadata"]["name"], phase=pod["status"]["phase"])

    def running(self):
        return self.phase == "Running"

    def succeeded(self):
        return self.phase == "Succeeded"

    def failed(self):
        return self.phase == "Failed"


def get_forge_jobs(shell: Shell) -> Generator[ForgeJob, None, None]:
    pod_result = shell.run([
        "kubectl", "get", "pods", "-n", "default", "-o", "json"
    ]).unwrap().decode()
    pods = json.loads(pod_result)["items"]
    for pod in pods:
        if pod["metadata"]["name"].startswith("forge-"):
            yield ForgeJob.from_pod(pod)


@main.command("list-jobs")
@click.option("--phase", multiple=True, help="Only show jobs in this phase")
@click.option("--regex", help="Only show jobs matching this regex")
def list_jobs(
    phase: List[str],
    regex: str,
) -> None:
    """List all available clusters"""
    shell = LocalShell()
    old_cluster = get_current_cluster_name(shell)
    pattern = re.compile(regex or ".*")
    try:
        for cluster in list_eks_clusters(shell):
            set_current_cluster(shell, cluster)
            print("Cluster:", cluster)
            for job in get_forge_jobs(shell):
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

                click.secho(f"{job.name} {job.phase}", fg=fg)
    except Exception as e:
        set_current_cluster(shell, old_cluster)
        raise e


if __name__ == "__main__":
    main()