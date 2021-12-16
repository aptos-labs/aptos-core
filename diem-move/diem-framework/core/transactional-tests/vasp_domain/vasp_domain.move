//# init --addresses Bob=0x7898b9021c0ed4af78fc7bd8e75b8e5d
//#      --private-keys Bob=9d631a6930d18ec34349f27bb7aaf7c4ea73435e3eb692c5401823bda76ac499

//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::DiemAccount;

    fun main() {
        assert!(!DiemAccount::exists_at(@Bob), 83);
    }
}

//# run --type-args 0x1::XUS::XUS
//#     --signers TreasuryCompliance
//#     --args 0u64 @Bob x"cad7c7c48f55dda47a1e2313d1ebab8a" b"bob" false
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::VASPDomain;

    fun main() {
        assert!(VASPDomain::tc_domain_manager_exists(), 77);
    }
}

// Add a diem id domain.
//# run --signers TreasuryCompliance --args @Bob b"diem" --show-events
//#     -- 0x1::TreasuryComplianceScripts::add_vasp_domain

// Check that the diem id domain is added to VASPDomains.
//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::VASPDomain;

    fun main() {
        assert!(VASPDomain::has_vasp_domain(@Bob, b"diem"), 5);
    }
}

// Add the same diem ID domain to the bob account, expect it to fail.
//# run --signers TreasuryCompliance --args @Bob b"diem" --show-events
//#     -- 0x1::TreasuryComplianceScripts::add_vasp_domain

// Check if the previously added domain is still there.
//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::VASPDomain;

    fun main() {
        assert!(VASPDomain::has_vasp_domain(@Bob, b"diem"), 5);
    }
}

// Remove a diem id domain.
//# run --signers TreasuryCompliance --args @Bob b"diem" --show-events
//#     -- 0x1::TreasuryComplianceScripts::remove_vasp_domain

// Check if diem id domain is removed from VASPDomains.
//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::VASPDomain;

    fun main() {
        assert!(!VASPDomain::has_vasp_domain(@Bob, b"diem"), 205);
    }
}

// Try adding a domain ID longer than 63 characters, expect to fail.
//# run --signers TreasuryCompliance --args @Bob b"aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffffgggg" --show-events
//#     -- 0x1::TreasuryComplianceScripts::remove_vasp_domain

// Check that the long domain ID is not added to VASPDomains.
//# run --signers DiemRoot DiemRoot --admin-script
script {
    use DiemFramework::VASPDomain;

    fun main() {
        assert!(!VASPDomain::has_vasp_domain(@Bob, b"aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffffgggg"), 888);
    }
}

// Check if vasp account tries to add domain id, it fails.
//# run --signers Bob --args @Bob b"bob_domain" --show-events
//#     -- 0x1::TreasuryComplianceScripts::add_vasp_domain
