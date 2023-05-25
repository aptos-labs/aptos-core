import unittest
from test_framework.shell import SpyShell, FakeCommand, RunResult
from test_framework.filesystem import SpyFilesystem, FILE_NOT_FOUND
from importlib.util import spec_from_loader, module_from_spec
from importlib.machinery import SourceFileLoader

import os
import imp

testrun = imp.load_source("testrun", os.path.join(os.path.dirname(__file__), "testrun"))


class TestTestrun(unittest.TestCase):
    def test_run_test_from_root(self):
        """Test that we can find the test file and run it from the root of the repo"""
        spy_shell = SpyShell(
            [
                FakeCommand(
                    "poetry -C testsuite run python3 -u testsuite/banana.py",
                    RunResult(0, b""),
                ),
            ]
        )
        spy_filesystem = SpyFilesystem(
            {}, {"testsuite/banana.py": b"", "banana.py": FILE_NOT_FOUND}
        )

        testrun.run_test(spy_shell, spy_filesystem, "banana.py")
        spy_shell.assert_commands(self)
        spy_filesystem.assert_reads(self)

    def test_run_test_from_testsuite(self):
        """Test that we can find the test file and run it from the testsuite directory"""
        spy_shell = SpyShell(
            [
                FakeCommand(
                    "poetry run python3 -u banana.py",
                    RunResult(0, b""),
                ),
            ]
        )
        spy_filesystem = SpyFilesystem({}, {"banana.py": b""})

        testrun.run_test(spy_shell, spy_filesystem, "banana.py")
        spy_shell.assert_commands(self)
        spy_filesystem.assert_reads(self)
