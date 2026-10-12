// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A script's function is verified as a module's: its module's name is
// quoted in the rendering (`«<SELF>_0»`), and must still be found.
script {
    fun main(x: u64) {
        assert!(x > 0, 1);
    }
    spec main {
        aborts_if false; // error: aborts at zero
    }
}
