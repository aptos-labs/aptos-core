// The `None` branch moves `value` into an unread temporary. Dead-instruction
// elimination removes the move even though the other branch reuses the
// temporary's slot.

// RUN: publish --print(stackless)
module 0x42::dce_dropped_reference {
    enum Maybe has drop {
        Some { inner: u64 },
        None,
    }

    fun contains(self: &Maybe, value: &u64): bool {
        if (self is Maybe::None) {
            false
        } else {
            &self.inner == value
        }
    }

    fun check(present: bool): bool {
        let maybe = if (present) Maybe::Some { inner: 7 } else Maybe::None;
        contains(&maybe, &7)
    }
}

// RUN: execute 0x42::dce_dropped_reference::check --args true
// CHECK: results: true

// RUN: execute 0x42::dce_dropped_reference::check --args false
// CHECK: results: false
