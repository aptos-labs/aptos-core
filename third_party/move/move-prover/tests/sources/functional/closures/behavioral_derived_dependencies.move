// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A body-derived ensures condition must register its spec-function dependencies,
// including their type instantiations, even when its abort condition is constant.
module 0x42::behavioral_derived_dependencies {
    fun identity<T: copy + drop>(x: T): T { x }
    spec identity {
        pragma opaque;
        aborts_if false;
        ensures result == model(x);
    }
    spec fun model<T>(x: T): T { x }

    fun wrapper<T: copy + drop>(x: T): T { identity(x) }

    fun check() {}
    spec check {
        ensures result_of<wrapper<u64>>(7) == 7;
        ensures result_of<wrapper<bool>>(true);
    }
}
