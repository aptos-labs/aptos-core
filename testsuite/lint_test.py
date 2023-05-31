import unittest
from unittest.mock import patch

from test_framework.shell import SpyShell, RunResult, FakeCommand
import lint
from lint import main

from click.testing import CliRunner


class HelmLintTestCase(unittest.TestCase):
    def testHelm(self) -> None:
        error = (
            b"[ERROR] templates/: parse error at (testnet-addons/templates/load"
            b"test.yaml:75): function alkajsdfl not defined"
        )
        shell = SpyShell(
            [
                FakeCommand("helm lint testsuite/fixtures/helm", RunResult(0, error)),
            ]
        )
        with patch.object(lint, "LocalShell", lambda *_: shell):
            runner = CliRunner()
            result = runner.invoke(
                main,
                ["--no-log-metadata", "helm", "testsuite/fixtures/helm"],
                catch_exceptions=False,
            )

        shell.assert_commands(self)
        expected_error = (
            "::error file=testsuite/fixtures/testnet-addons/templates/loadtest."
            "yaml,line=75,col=1::function alkajsdfl not defined\n"
        )
        self.assertEqual(result.output, expected_error)
