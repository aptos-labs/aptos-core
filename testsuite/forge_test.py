from importlib.metadata import files
import unittest
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, OrderedDict, Sequence, Union

from click.testing import CliRunner
from .forge import AwsError, ForgeFormatter, ForgeResult, ForgeState, K8sForgeRunner, assert_aws_token_expiration, main, ForgeContext, LocalForgeRunner, FakeShell, FakeFilesystem, RunResult, FakeProcesses


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


class SpyFilesystem(FakeFilesystem):
    def __init__(self, expected_writes: Dict[str, bytes], expected_reads: Dict[str, bytes]) -> None:
        self.expected_writes = expected_writes
        self.expected_reads = expected_reads
        self.writes = {}
        self.reads = []
        self.temp_count = 1

    def write(self, filename: str, contents: bytes) -> None:
        self.writes[filename] = contents

    def read(self, filename: str) -> bytes:
        self.reads.append(filename)
        return self.expected_reads.get(filename, b"")

    def assert_writes(self, testcase) -> None:
        for filename, contents in self.expected_writes.items():
            testcase.assertIn(filename, self.writes, f"{filename} was not written: {self.writes}")
            testcase.assertMultiLineEqual(self.writes[filename].decode(), contents.decode(), f"{filename} did not match expected contents")

    def assert_reads(self, testcase) -> None:
        for filename in self.expected_reads.keys():
            testcase.assertIn(filename, self.reads, f"{filename} was not read")

    def mkstemp(self) -> str:
        filename = f"temp{self.temp_count}"
        self.temp_count += 1
        return filename


def fake_context(shell=None, filesystem=None, processes=None) -> ForgeContext:
    return ForgeContext(
        shell=shell if shell else FakeShell(),
        filesystem=filesystem if filesystem else FakeFilesystem(),
        processes=processes if processes else FakeProcesses(),
        epoch="123456",

        forge_test_suite="banana",
        local_p99_latency_ms_threshold="6000",
        forge_runner_tps_threshold="593943",
        forge_runner_duration_secs="123",

        reuse_args=[],
        keep_args=[],
        haproxy_args=[],

        aws_account_num="123",
        aws_region="banana-east-1",

        forge_image_tag="asdf",
        forge_namespace="potato",

        github_actions="false",
    )


class ForgeRunnerTests(unittest.TestCase):
    def testLocalRunner(self) -> None:
        shell = SpyShell({
            'cargo run -p forge-cli -- --suite banana --mempool-backlog 5000 '
            '--avg-tps 593943 --max-latency-ms 6000 --duration-secs 123 test '
            'k8s-swarm --image-tag asdf --namespace potato --port-forward':
            RunResult(0, b"orange"),
        })
        filesystem = SpyFilesystem({}, {})
        context = fake_context(shell, filesystem)
        runner = LocalForgeRunner()
        result = runner.run(context)
        self.assertEqual(result.state, ForgeState.PASS, result.output)
        shell.assert_commands(self)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)

    def testK8sRunner(self) -> None:
        self.maxDiff = None
        shell = SpyShell(OrderedDict([
            ("kubectl delete pod -n default -l forge-namespace=potato --force", RunResult(0, b"")),
            ("kubectl wait -n default --for=delete pod -l forge-namespace=potato", RunResult(0, b"")),
            ("kubectl apply -n default -f temp1", RunResult(0, b"")),
            ("kubectl wait -n default --timeout=5m --for=condition=Ready pod/potato-123456-asdf", RunResult(0, b"")),
            ("kubectl logs -n default -f potato-123456-asdf", RunResult(0, b"")),
            ("kubectl get pod -n default potato-123456-asdf -o jsonpath='{.status.phase}'", RunResult(0, b"Succeeded")),
        ]))
        cwd = Path(__file__).absolute().parent
        forge_yaml = cwd / "forge-test-runner-template.yaml"
        template_fixture = cwd / "forge-test-runner-template.fixture"
        filesystem = SpyFilesystem({
            "temp1": template_fixture.read_bytes(),
        }, {
            "testsuite/forge-test-runner-template.yaml": forge_yaml.read_bytes(),
        })
        context = fake_context(shell, filesystem)
        runner = K8sForgeRunner()
        result = runner.run(context)
        self.assertEqual(result.state, ForgeState.PASS, result.output)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)


class TestAWSTokenExpiration(unittest.TestCase):
    def testNoAwsToken(self) -> None:
        with self.assertRaisesRegex(AwsError, "AWS token is required"): 
            assert_aws_token_expiration(None)

    def testAwsTokenExpired(self) -> None:
        expiration = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S%z")
        with self.assertRaisesRegex(AwsError, "AWS token has expired"):
            assert_aws_token_expiration(expiration)
    
    def testAwsTokenMalformed(self) -> None:
        with self.assertRaisesRegex(AwsError, "Invalid date format:.*"):
            assert_aws_token_expiration("asdlkfjasdlkjf")


class ForgeFormattingTests(unittest.TestCase):
    def testReport(self):
        filesystem = SpyFilesystem({"test": b"banana"}, {})
        context = fake_context(filesystem=filesystem)
        result = ForgeResult(ForgeState.PASS, "test")
        context.report(result, [
            ForgeFormatter("test", lambda c, r: "banana")
        ])
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)
