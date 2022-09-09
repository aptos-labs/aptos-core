import unittest

from determinator import ChangedFilesPredicate, ChangedFilesContext


class ChangedFilesPredicateTestCase(unittest.TestCase):
    def test_changed_files_passes(self) -> None:
        context: ChangedFilesContext = {
            "changed_files": ["asdf"]
        }
        predicate = ChangedFilesPredicate(["a.*f"])
        verdict = predicate.evaluate(context)
        self.assertTrue(verdict.verdict, verdict.reason)

    def test_changed_files_fails(self) -> None:
        context: ChangedFilesContext = {
            "changed_files": ["asdf"]
        }
        predicate = ChangedFilesPredicate(["fdas"])
        verdict = predicate.evaluate(context)
        self.assertFalse(verdict.verdict, verdict.reason)
