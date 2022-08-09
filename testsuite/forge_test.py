import unittest
from click.testing import CliRunner
from typing import Dict, OrderedDict, Sequence, Union

from .forge import K8sForgeRunner, main, ForgeContext, LocalForgeRunner, FakeShell, FakeWriter, RunResult, FakeProcesses


class SpyShell(FakeShell):
    def __init__(self, command_map: Dict[str, Union[RunResult, Exception]]) -> None:
        self.command_map = command_map
        self.commands = []

    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        result = self.command_map.get(" ".join(command), super().run(command))
        self.commands.append(" ".join(command))
        if isinstance(result, Exception):
            raise result
        return result

    def assert_commands(self, testcase) -> None:
        testcase.assertEqual(list(self.command_map.keys()), self.commands)


class SpyWriter(FakeWriter):
    def __init__(self, expected_writes: Dict[str, bytes]) -> None:
        self.expected_writes = expected_writes
        self.writes = {}

    def write(self, filename: str, contents: bytes) -> None:
        self.writes[filename] = contents

    def assert_writes(self, testcase) -> None:
        for filename, contents in self.expected_writes.items():
            testcase.assertEqual(self.writes[filename], contents, f"{filename} did not match expected contents")


def fake_context(shell=None, writer=None, processes=None) -> ForgeContext:
    return ForgeContext(
        shell=shell if shell else FakeShell(),
        writer=writer if writer else FakeWriter(),
        processes=processes if processes else FakeProcesses(),
        test_suite="banana",
        local_p99_latency_ms_threshold="6000",
        forge_runner_tps_threshold="593943",
        forge_runner_duration_secs="123",

        reuse_args=[],
        keep_args=[],
        haproxy_args=[],

        forge_output="apple",
        forge_image_tag="asdf",
        forge_namespace="potato",

        github_actions="false",
    )


class ForgeTests(unittest.TestCase):
    def testLocalRunner(self) -> None:
        shell = SpyShell({
            'cargo run -p forge-cli -- --suite banana --mempool-backlog 5000 '
            '--avg-tps 593943 --max-latency-ms 6000 --duration-secs 123 test '
            'k8s-swarm --image-tag asdf --namespace potato --port-forward':
            RunResult(0, b'orange'),
        })
        writer = SpyWriter({
            "apple": b"orange",
        })
        context = fake_context(shell, writer)
        runner = LocalForgeRunner()
        runner.run(context)
        shell.assert_commands(self)
        writer.assert_writes(self)

    def testK8sRunner(self) -> None:
        shell = SpyShell(OrderedDict([
            ('kubectl delete pod -n default -l forge-namespace=potato --force', RunResult(0, b"")),
            ('kubectl wait -n default --for=delete pod -l forge-namespace=potato', RunResult(0, b"")),
        ]))
        writer = SpyWriter({
        })
        context = fake_context(shell, writer)
        runner = K8sForgeRunner()
        runner.run(context)
        shell.assert_commands(self)
        writer.assert_writes(self)