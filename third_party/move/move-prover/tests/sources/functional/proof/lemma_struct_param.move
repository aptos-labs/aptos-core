// A lemma may take a struct of its own module, or of another, as a parameter.
//
// Lemma signatures are pre-registered before any module is analyzed, so that a
// proof in one module can apply a lemma of another whatever the processing
// order. That pass translated the signature before the structs it names were
// declared, and a lemma over `Account` failed with "undeclared `Account`".
module 0x42::lemma_struct_param {
    struct Account has copy, drop { balance: u64, limit: u64 }

    spec module {
        fun within_limit(a: Account): bool { a.balance <= a.limit }

        lemma raise_limit_keeps(a: Account, extra: u64) {
            requires within_limit(a);
            requires a.limit + extra <= MAX_U64;
            ensures within_limit(Account { balance: a.balance, limit: a.limit + extra });
        }
    }

    public fun raise_limit(a: Account, extra: u64): Account {
        Account { balance: a.balance, limit: a.limit + extra }
    }
    spec raise_limit {
        requires a.balance <= a.limit;
        requires a.limit + extra <= MAX_U64;
        ensures within_limit(result);
    } proof {
        apply raise_limit_keeps(a, extra);
    }
}

module 0x42::lemma_struct_param_user {
    use 0x42::lemma_struct_param::{Self as lsp, Account};

    fun raise_twice(a: Account, extra: u64): Account {
        let once = lsp::raise_limit(a, extra);
        lsp::raise_limit(once, extra)
    }
    spec raise_twice {
        requires a.balance <= a.limit;
        requires a.limit + extra + extra <= MAX_U64;
        ensures lsp::within_limit(result);
    }
}
