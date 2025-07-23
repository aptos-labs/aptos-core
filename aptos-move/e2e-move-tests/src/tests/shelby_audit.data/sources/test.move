module my_addr::test {
    use std::vector;
    use std::signer;

    struct AuditReport has key, store, drop {
        data: vector<u8>,
    }

    fun new_audit_report(fill: u8): AuditReport {
        let v = vector::empty();
        for (i in 0..65536) {
            v.push_back(fill)
        };

        AuditReport {
            data: v,
        }
    }

    entry fun store_new_audit_report(s: signer, fill: u8) acquires AuditReport {
        let addr = signer::address_of(&s);
        if (exists<AuditReport>(addr)) {
            let r = borrow_global_mut<AuditReport>(addr);
            *r = new_audit_report(fill);
        }
        else {
            move_to(&s, new_audit_report(fill));
        }
    }
}
