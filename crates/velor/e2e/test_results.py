# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import logging
import typing
from dataclasses import dataclass, field
from functools import wraps

LOG = logging.getLogger(__name__)


# This class holds info about passed / failed tests.
@dataclass(init=True)
class TestResults:
    passed: typing.List[str] = field(default_factory=list)
    failed: typing.List[typing.Tuple[str, Exception]] = field(default_factory=list)


# This is a decorator that you put above every test case. It handles capturing test
# success / failure so it can be reported at the end of the test suite. It also handles
# passing in test_name based on the name of the function so the caller doesn't have to.
def build_test_case_decorator(test_results: TestResults):
    def test_case_inner(f):
        @wraps(f)
        def wrapper(*args, **kwds):
            LOG.info(f"Running test: {f.__name__}")
            try:
                result = f(*args, test_name=f.__name__, **kwds)
                test_results.passed.append(f.__name__)
                return result
            except Exception as e:
                test_results.failed.append((f.__name__, e))
                return None

        return wrapper

    return test_case_inner


# We now define one TestResults that we'll use for every test case. This is a bit of a
# hack but it is the only way to then be able to provide a decorator that works out of
# the box. The alternative was to use a context manager and wrap every function call in
# it, but not only is that more verbose, but you'd have to provide the name of each test
# case manually to the context manager, whereas with this approach the name can be
# inferred from the function being decorated directly.
test_results = TestResults()

# Then we define an instance of the decorator that uses that TestResults instance.
test_case = build_test_case_decorator(test_results)
