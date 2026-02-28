// flag: --language-version=2.4
module 0x42::proof_hint_split {

    // ============================================================
    // Split on bool expression.

    fun abs_diff(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { b - a }
    }
    spec abs_diff {
        ensures result == if (a >= b) { a - b } else { b - a };

        proof {
            split on a >= b;
        }
    }

    // ============================================================
    // Split on enum: one variant per case.

    enum Color has drop { Red, Green, Blue }

    fun color_code(c: Color): u64 {
        match (c) {
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
        }
    }
    spec color_code {
        ensures result >= 1 && result <= 3;

        proof {
            split on c;
        }
    }

    enum Toggle has drop { On, Off }

    fun is_on(t: Toggle): bool {
        match (t) {
            Toggle::On => true,
            Toggle::Off => false,
        }
    }
    spec is_on {
        ensures result == (t is On);

        proof {
            split on t;
        }
    }
}
