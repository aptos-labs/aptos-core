// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `assert` in proof blocks.
module 0x42::proof_assert {

    // Assert an intermediate fact about the result.
    fun double(x: u64): u64 {
        x + x
    }
    spec double {
        requires x + x <= MAX_U64;
        ensures result == 2 * x;
    } proof {
        post assert result == x + x;
        assert x + x == 2 * x;
    }

    // Assert with spec function reference.
    spec fun is_positive(x: u64): bool { x > 0 }

    fun increment(x: u64): u64 {
        x + 1
    }
    spec increment {
        requires x < MAX_U64;
        ensures is_positive(result);
    } proof {
        post assert result == x + 1;
        assert x + 1 > 0;
        assert is_positive(x + 1);
    }

    // Assert in a multi-step computation.
    fun weighted_sum(a: u64, b: u64): u64 {
        2 * a + 3 * b
    }
    spec weighted_sum {
        requires 2 * a + 3 * b <= MAX_U64;
        ensures result == 2 * a + 3 * b;
    } proof {
        assert 2 * a <= 2 * a + 3 * b;
        post assert result == 2 * a + 3 * b;
    }

    // ==================================================================
    // FAILURE: False proof assertion.
    // The post assert `result > 100` cannot hold for small inputs.

    fun successor(x: u64): u64 {
        x + 1
    }
    spec successor {
        requires x < MAX_U64;
        ensures result == x + 1;
    } proof {
        post assert result > 100;  // error: x could be 0, so result == 1
    }
}
