// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Note: If this test file fails to run, it is possible that the
// compiled version of the Move stdlib needs to be updated. This code
// is compiled with the latest compiler and stdlib, but it runs with
// the compiled stdlib.

address {{sender}} {

module MyModule {
    use AptosFramework::Aptos::Aptos;

    // The identity function for coins: takes a Aptos<T> as input and hands it back
    public fun id<T>(c: Aptos<T>): Aptos<T> {
        c
    }
}

}
