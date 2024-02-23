module test {
    fun f(x: u64): u64 {
        if (x == 1) {
            if (x == 11) { 11 } else
                if (x == 12) { 12 } else
                    if (x == 13) { 13 } else { 14 }
        } else
            if (x == 2) {
                if (x == 21) { 21 } else
                    if (x == 22) { 22 } else
                        if (x == 23) { 23 } else { 24 }
            } else
                if (x == 3) { 4 } else {
                    if (x == 51) { 51 } else
                        if (x == 52) { 52 } else
                            if (x == 53) { 53 } else { 54 }
                }
    }
}