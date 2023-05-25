import unittest
import tempfile
import os

from click.testing import CliRunner
from determinator import main, ChangedFilesPredicate, ChangedFilesContext


# If this test is run from Github Actions, $GITHUB_OUTPUT will be set
# Otherwise, create a tempfile for it, which exists for the duration of the test
if "GITHUB_OUTPUT" not in os.environ:
    temp_github_output = tempfile.NamedTemporaryFile(
        prefix="determinator_github_output_"
    )
    os.environ["GITHUB_OUTPUT"] = temp_github_output.name
print(f"{__file__}: GITHUB_OUTPUT set to {os.environ['GITHUB_OUTPUT']}")


class ChangedFilesPredicateTestCase(unittest.TestCase):
    def test_changed_files_passes(self) -> None:
        context: ChangedFilesContext = {"changed_files": ["asdf"]}
        predicate = ChangedFilesPredicate(["a.*f"])
        verdict = predicate.evaluate(context)
        self.assertTrue(verdict.verdict, verdict.reason)

    def test_changed_files_fails(self) -> None:
        context: ChangedFilesContext = {"changed_files": ["asdf"]}
        predicate = ChangedFilesPredicate(["fdas"])
        verdict = predicate.evaluate(context)
        self.assertFalse(verdict.verdict, verdict.reason)


class DeterminatorTestCase(unittest.TestCase):
    def test_determinator_from_github(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "changed-files",
                "--github-output-key",
                "BANANA",
                "testsuite/fixtures/helm",
            ],
            catch_exceptions=False,
        )
        self.assertEqual(
            result.output,
            "FAILED because Matched files: []\n" "BANANA=false\n",
        )
        self.assertEqual(result.exit_code, 0)

    def test_determinator_from_github_passes(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "changed-files",
                "--pattern",
                ".*/.*.ts",
                "--github-output-key",
                "BANANA",
                "testsuite/fixtures/helm/banana.ts",
            ],
            catch_exceptions=False,
        )
        self.assertEqual(
            result.output,
            "PASSED because Matched files: "
            "['testsuite/fixtures/helm/banana.ts']\n"
            "BANANA=true\n",
        )
        self.assertEqual(result.exit_code, 0)
