module NamedAddr::Detector {
    public fun test_loops() {
        // Loop with an unconditional break
        loop {
            if (condition()) {
                break;
            }
           
        };

        // Loop with a conditional break
        loop {
            if (condition()) {
                // Some code...
            } else {
                break;
            }
           
        };

        // Loop with a return
        loop {
            if (condition()) {
                return;
            }
           
        };

        // Nested loop where inner loop has an unconditional break
        loop {
            loop {
                break;
            }
           
        };

        // A more complex loop with nested conditions
        loop {
            if (condition()) {
                if (another_condition()) {
                    continue;
                } else {
                    return;
                }
            } else {
                break;
            }
           
        };
    }

    fun condition(): bool {
        true
    }

    fun another_condition(): bool {
        false
    }
}