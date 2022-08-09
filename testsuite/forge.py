import multiprocessing
import os
import pwd
import resource
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Dict, Generator, Optional, Sequence


@dataclass
class RunResult:
    exit_code: int
    output: bytes

    def unwrap(self) -> bytes:
        if self.exit_code != 0:
            raise Exception(self.output.decode("utf-8"))
        return self.output


class Shell:
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        raise NotImplementedError


class LocalShell(Shell):
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        # Write to a temp file, stream to stdout
        tmpname = tempfile.mkstemp()[1]
        print("Writing to", tmpname)
        with open(tmpname, 'wb') as writer, open(tmpname, 'rb') as reader:
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
    answer = os.getenv("FORGE_INSTALL_DEPENDENCIES")
    if not answer:
        answer = input("Would you like to install it now? (y/n) ").strip().lower()
    if answer in ("y", "yes", "yeet", "yessir", "si"):
        shell = LocalShell()
        shell.run(["pip3", "install", dependency]).unwrap()
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


def get_current_epoch() -> str:
    return datetime.now().strftime('%s')


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


class Writer:
    def write(self, filename: str, contents: bytes) -> None:
        raise NotImplementedError()


class FakeWriter(Writer):
    def write(self, filename: str, contents: bytes) -> None:
        print(f"Wrote {contents} to {filename}")


class FileWriter(Writer):
    def write(self, filename: str, contents: bytes) -> None:
        with open(filename, 'wb') as f:
            f.write(contents)

class Reader:
    def read(self, filename: str) -> bytes:
        raise NotImplementedError()


class FileReader(Reader):
    def read(self, filename: str) -> bytes:
        with open(filename, 'rb') as f:
            return f.read()


# o11y resources
INTERN_ES_DEFAULT_INDEX = "90037930-aafc-11ec-acce-2d961187411f"
INTERN_ES_BASE_URL = "https://es.intern.aptosdev.com"
INTERN_GRAFANA_BASE_URL = "https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&var-Datasource=Remote%20Prometheus%20Intern"
DEVINFRA_ES_DEFAULT_INDEX = "d0bc5e20-badc-11ec-9a50-89b84ac337af"
DEVINFRA_ES_BASE_URL = "https://es.devinfra.aptosdev.com"
DEVINFRA_GRAFANA_BASE_URL = "https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&var-Datasource=Remote%20Prometheus%20Devinfra"
HUMIO_LOGS_LINK = "https://cloud.us.humio.com/k8s/search?query=%24forgeLogs%28validator_instance%3Dvalidator-0%29%20%7C%20$FORGE_NAMESPACE%20&live=true&start=24h&widgetType=list-view&columns=%5B%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22%40timestamp%22%2C%22format%22%3A%22timestamp%22%2C%22width%22%3A180%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22level%22%2C%22format%22%3A%22text%22%2C%22width%22%3A54%7D%2C%7B%22type%22%3A%22link%22%2C%22openInNewBrowserTab%22%3Atrue%2C%22style%22%3A%22button%22%2C%22hrefTemplate%22%3A%22https%3A%2F%2Fgithub.com%2Faptos-labs%2Faptos-core%2Fpull%2F%7B%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22textTemplate%22%3A%22%7B%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22header%22%3A%22Forge%20PR%22%2C%22width%22%3A79%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.namespace%22%2C%22format%22%3A%22text%22%2C%22width%22%3A104%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.pod_name%22%2C%22format%22%3A%22text%22%2C%22width%22%3A126%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.container_name%22%2C%22format%22%3A%22text%22%2C%22width%22%3A85%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22message%22%2C%22format%22%3A%22text%22%7D%5D&newestAtBottom=true&showOnlyFirstLine=false"


def prometheus_port_forward() -> None:
    os.execvp("kubectl", ["kubectl", "port-forward", "prometheus", "9090"])


class Process:
    def name(self) -> str:
        raise NotImplementedError()
    
    def kill(self) -> None:
        raise NotImplementedError()


@dataclass
class FakeProcess(Process):
    _name: str

    def name(self) -> str:
        return self._name

    def kill(self) -> None:
        print("killing {self._name}")


class Processes:
    def processes(self) -> Generator[Process, None, None]:
        raise NotImplementedError()


@dataclass
class SystemProcess(Process):
    process: psutil.Process

    def name(self) -> str:
        return self.process.name()

    def kill(self) -> None:
        self.process.kill()


class SystemProcesses:
    def processes(self) -> Generator[Process, None, None]:
        for process in psutil.process_iter():
            yield SystemProcess(process)


class FakeProcesses(Processes):
    def processes(self) -> Generator[Process, None, None]:
        yield SystemProcess(psutil.Process(os.getpid()))


@dataclass
class ForgeContext:
    shell: Shell
    writer: Writer
    processes: Processes
    test_suite: str

    local_p99_latency_ms_threshold: str
    forge_runner_tps_threshold: str
    forge_runner_duration_secs: str
    forge_namespace: str
    
    # Cluster options
    reuse_args: Sequence[str]
    keep_args: Sequence[str]
    haproxy_args: Sequence[str]

    forge_output: str
    forge_image_tag: str
    forge_namespace: str

    github_actions: str

    def write(self, filename: str, content: bytes) -> None:
        self.writer.write(filename, content)


class ForgeRunner:
    def run(self, context: ForgeContext) -> None:
        raise NotImplementedError


class LocalForgeRunner(ForgeRunner):
    def run(self, context: ForgeContext) -> None:
        # Set rlimit to unlimited
        resource.setrlimit(resource.RLIMIT_NOFILE, (resource.RLIM_INFINITY, resource.RLIM_INFINITY))
        # Using fork can crash the subprocess, use spawn instead
        multiprocessing.set_start_method('spawn')
        port_forward_process = multiprocessing.Process(daemon=True, target=prometheus_port_forward)
        port_forward_process.start()
        result = context.shell.run([
            "cargo", "run", "-p", "forge-cli",
            "--",
            "--suite", context.test_suite,
            "--mempool-backlog", "5000",
            "--avg-tps", context.forge_runner_tps_threshold,
            "--max-latency-ms", context.local_p99_latency_ms_threshold,
            "--duration-secs", context.forge_runner_duration_secs,
            "test", "k8s-swarm",
            "--image-tag", context.forge_image_tag,
            "--namespace", context.forge_namespace,
            "--port-forward",
            *context.reuse_args,
            *context.keep_args,
            *context.haproxy_args,
        ], stream_output=True)
        # Todo write forge output to file
        context.write(context.forge_output, result.unwrap())
        # Kill port forward unless we're keeping them
        if not context.keep_args:
            # Kill all processess with kubectl in the name
            for process in psutil.process_iter():
                if 'kubectl' in process.name():
                    process.kill()
            port_forward_process.terminate()
            port_forward_process.join()


class K8sForgeRunner(ForgeRunner):
    def run(self, context: ForgeContext) -> None:
        forge_pod_name = f"{context.forge_namespace}-{get_current_epoch()}-{context.forge_image_tag}"[:64]
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

        # Make a temp spec file
        # Determine triggered by...? GITHUB ACTION
        # Sed the template file
        # Kubectl apply specfile
        # Wait for thing to come online
        # Tail the logs
        # Parse pod status

        """

        specfile=$(mktemp)
        echo "Forge test-runner pod Spec : ${specfile}"

        [[ "$GITHUB_ACTIONS" == "true" ]] && FORGE_TRIGGERED_BY=github-actions || FORGE_TRIGGERED_BY=other

        sed -e "s/{FORGE_POD_NAME}/${FORGE_POD_NAME}/g" \
            -e "s/{FORGE_TEST_SUITE}/${FORGE_TEST_SUITE}/g" \
            -e "s/{FORGE_RUNNER_DURATION_SECS}/${FORGE_RUNNER_DURATION_SECS}/g" \
            -e "s/{FORGE_RUNNER_TPS_THRESHOLD}/${FORGE_RUNNER_TPS_THRESHOLD}/g" \
            -e "s/{IMAGE_TAG}/${IMAGE_TAG}/g" \
            -e "s/{AWS_ACCOUNT_NUM}/${AWS_ACCOUNT_NUM}/g" \
            -e "s/{AWS_REGION}/${AWS_REGION}/g" \
            -e "s/{FORGE_NAMESPACE}/${FORGE_NAMESPACE}/g" \
            -e "s/{REUSE_ARGS}/${REUSE_ARGS}/g" \
            -e "s/{KEEP_ARGS}/${KEEP_ARGS}/g" \
            -e "s/{ENABLE_HAPROXY_ARGS}/${ENABLE_HAPROXY_ARGS}/g" \
            -e "s/{FORGE_TRIGGERED_BY}/${FORGE_TRIGGERED_BY}/g" \
            testsuite/forge-test-runner-template.yaml >${specfile}

        kubectl apply -n default -f $specfile

        # wait for enough time for the pod to start and potentially new nodes to come online
        kubectl wait -n default --timeout=5m --for=condition=Ready "pod/${FORGE_POD_NAME}"

        # tail the logs and tee them for further parsing
        kubectl logs -n default -f $FORGE_POD_NAME | tee $FORGE_OUTPUT

        # parse the pod status: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
        forge_pod_status=$(kubectl get pod -n default $FORGE_POD_NAME -o jsonpath="{.status.phase}" 2>&1)
        echo "Forge pod status: ${forge_pod_status}"

        if [ "$forge_pod_status" = "Succeeded" ]; then # the current pod succeeded
            FORGE_EXIT_CODE=0
        elif echo $forge_pod_status | grep -E "(not found)|(NotFound)"; then # the current test in this namespace was likely preempted and deleted
            FORGE_EXIT_CODE=10
        else # it did not succeed
            FORGE_EXIT_CODE=1
        fi
        """


@main.command()
# for calculating regression in local mode
@envoption("LOCAL_P99_LATENCY_MS_THRESHOLD", "60000")
# output files
@envoption("FORGE_OUTPUT", lambda: tempfile.mkstemp()[1])
@envoption("FORGE_REPORT", lambda: tempfile.mkstemp()[1])
@envoption("FORGE_PRE_COMMENT")
@envoption("FORGE_COMMENT")
# cluster auth
@envoption("AWS_ACCOUNT_NUM")
@envoption("AWS_REGION", "us-west-2")
# forge test runner customization
@envoption("FORGE_RUNNER_MODE", "k8s")
@envoption("FORGE_NAMESPACE_KEEP")
@envoption("FORGE_NAMESPACE_REUSE")
@envoption("FORGE_ENABLE_HAPROXY")
@envoption("FORGE_TEST_SUITE", "land_blocking")
@envoption("FORGE_RUNNER_DURATION_SECS", "300")
@envoption("FORGE_RUNNER_TPS_THRESHOLD", "400")
@envoption("IMAGE_TAG", "devnet")
@envoption("FORGE_NAMESPACE", f"forge-{get_current_user()}-{get_current_epoch()}")
@envoption("GITHUB_ACTIONS", "false")
@click.option("--dry-run", is_flag=True)
def test(
    local_p99_latency_ms_threshold: int,
    forge_output: Optional[str],
    forge_report: Optional[str],
    forge_pre_comment: Optional[str],
    forge_comment: Optional[str],
    aws_account_num: Optional[str],
    aws_region: str,
    forge_runner_mode: str,
    forge_namespace_keep: Optional[str],
    forge_namespace_reuse: Optional[str],
    forge_enable_haproxy: Optional[str],
    forge_test_suite: str,
    forge_runner_duration_secs: str,
    forge_runner_tps_threshold: str,
    image_tag: Optional[str],
    forge_namespace: Optional[str],
    github_actions: str,
    dry_run: Optional[bool],
) -> None:
    shell = FakeShell() if dry_run else LocalShell()
    writer = FakeWriter() if dry_run else FileWriter()
    processes = FakeProcesses() if dry_run else SystemProcesses()
    # Make temp files for forge output
    # Make sure we're authorized for aws
    # Set args for forge
    # Set forge namespace
    # Set image tag
    # Set o11y resource locations
    # Run forge
    context = ForgeContext(
        shell=shell,
        writer=writer,
        processes=processes,
        test_suite=forge_test_suite,
        local_p99_latency_ms_threshold=local_p99_latency_ms_threshold,
        forge_runner_tps_threshold=forge_runner_tps_threshold,
        forge_runner_duration_secs=forge_runner_duration_secs,

        reuse_args=["--reuse"] if forge_namespace_reuse else [],
        keep_args=["--keep"] if forge_namespace_keep else [],
        haproxy_args=["--enable-haproxy"] if forge_enable_haproxy else [],

        forge_output=forge_output,
        forge_image_tag=image_tag,
        forge_namespace=forge_namespace,

        github_actions=github_actions,
    )
    forge_runner_mapping = {
        'local': LocalForgeRunner,
        'k8s': K8sForgeRunner,
    }
    forge_runner = forge_runner_mapping[forge_runner_mode]()
    forge_runner.run(context)


if __name__ == "__main__":
    main()