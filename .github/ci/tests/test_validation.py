"""Tests for ci_actions.validation: the strict JSON loader and shared predicates."""

import json
import unittest
from pathlib import Path

from hypothesis import given, strategies as st

from tests.property_support import configure_profiles

configure_profiles()

from ci_actions import validation
from ci_actions.validation import (
    MAX_SAFE_INTEGER, REPOSITORY, SHA40, DuplicateKeyError, NonFiniteNumberError,
    is_safe_positive_int, loads_strict,
)

JSON_VALUES = st.recursive(
    st.none() | st.booleans() | st.integers(-(2**63), 2**63)
    | st.floats(allow_nan=False, allow_infinity=False) | st.text(max_size=8),
    lambda children: st.lists(children, max_size=4) | st.dictionaries(st.text(max_size=8), children, max_size=4),
    max_leaves=12,
)


class LoadsStrictTests(unittest.TestCase):
    def test_rejects_duplicate_keys_at_any_depth(self):
        for text, key in (('{"a": 1, "a": 2}', "a"), ('[{"b": {"c": 1, "c": 1}}]', "c")):
            with self.subTest(text=text), self.assertRaises(DuplicateKeyError) as caught:
                loads_strict(text)
            self.assertEqual(f"duplicate key: {key}", str(caught.exception))
            self.assertIsInstance(caught.exception, ValueError)

    def test_rejects_non_finite_numbers(self):
        for token in ("NaN", "Infinity", "-Infinity", "1e309", "-1e309"):
            with self.subTest(token=token), self.assertRaises(NonFiniteNumberError) as caught:
                loads_strict(f"[{token}]")
            self.assertEqual(f"JSON number must be finite: {token}", str(caught.exception))
            self.assertIsInstance(caught.exception, ValueError)

    def test_rejects_invalid_utf8_and_byte_order_marks(self):
        with self.assertRaises(UnicodeDecodeError):
            loads_strict(b'"\xff"')
        for data in (b"\xef\xbb\xbf[1]", "﻿[1]", "[1]".encode("utf-16")):
            with self.subTest(data=data), self.assertRaises(ValueError):
                loads_strict(data)

    @given(JSON_VALUES)
    def test_round_trips_finite_json_values(self, value):
        for text in (json.dumps(value), json.dumps(value, ensure_ascii=False)):
            for data in (text, text.encode("utf-8"), bytearray(text.encode("utf-8"))):
                self.assertEqual(value, loads_strict(data))

    def test_rejects_excessive_nesting_as_value_error(self):
        for data in ("[" * 100_000 + "]" * 100_000, ("[" * 100_000 + "]" * 100_000).encode()):
            with self.subTest(kind=type(data).__name__), self.assertRaisesRegex(ValueError, "JSON nesting is too deep"):
                loads_strict(data)


class SharedPredicateTests(unittest.TestCase):
    def test_safe_positive_int_bounds_and_types(self):
        self.assertEqual(2**53 - 1, MAX_SAFE_INTEGER)
        for value in (1, 42, MAX_SAFE_INTEGER):
            self.assertTrue(is_safe_positive_int(value), value)
        for value in (0, -1, MAX_SAFE_INTEGER + 1, True, False, 1.0, "1", None):
            self.assertFalse(is_safe_positive_int(value), repr(value))

    def test_patterns_match_only_exact_values(self):
        self.assertTrue(SHA40.fullmatch("a" * 40))
        for bad in ("A" * 40, "a" * 39, "a" * 41, "g" * 40):
            self.assertIsNone(SHA40.fullmatch(bad), bad)
        match = REPOSITORY.fullmatch("aptos-labs/aptos-core")
        self.assertEqual(("aptos-labs", "aptos-core"), match.groups())
        for bad in ("aptos-labs", "a/b/c", "a b/c", "/c"):
            self.assertIsNone(REPOSITORY.fullmatch(bad), bad)

    def test_module_has_no_ci_actions_dependencies(self):
        source = Path(validation.__file__).read_text(encoding="utf-8")
        self.assertNotIn("from ci_actions", source)
        self.assertNotIn("import ci_actions", source)


class SingleDefinitionTests(unittest.TestCase):
    """Every ci_actions module uses ci_actions.validation instead of a copy."""

    NEEDLES = ("2**53", "[0-9a-f]{40}", "[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", "[A-Za-z0-9_.-]+)/(",
               "json.loads(", "json.load(", "object_pairs_hook", "parse_constant", "is_positive_id")

    def test_no_module_redefines_shared_validation(self):
        root = Path(validation.__file__).parent
        modules = sorted(path for path in root.glob("*.py") if path.name != "validation.py")
        self.assertTrue(modules)
        for path in modules:
            text = path.read_text(encoding="utf-8")
            for needle in self.NEEDLES:
                with self.subTest(module=path.name, needle=needle):
                    self.assertNotIn(needle, text)


if __name__ == "__main__":
    unittest.main()
