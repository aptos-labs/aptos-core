//# init --addresses alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key alice
module alice::randomness_user {
    use velor_framework::randomness;

    fun uses_randomness(): u8 {
        randomness::u8_integer()
    }

    #[lint::allow_unsafe_randomness]
    /// Allow to export randomness user for testing
    public fun uses_randomness_indirect(): u8 {
        uses_randomness()
    }
}

//# publish --private-key alice
module alice::randomness_test {
    use alice::randomness_user;

    // Expecting error because public function calls randomness
    public fun randomness_error(): u8 {
        randomness_user::uses_randomness_indirect()
    }

    // Ok because randomness caller is not public
    public(friend) fun randomness_ok_since_not_public(): u8 {
        randomness_user::uses_randomness_indirect()
    }

    // Ok because randomness caller is exempted
    #[lint::allow_unsafe_randomness]
    public fun randomness_ok_since_overridden(): u8 {
        randomness_user::uses_randomness_indirect()
    }

    // Error because non-public entry does not have randomness attribute
    public(friend) entry fun missing_randomness_attribute(_s: &signer) {
        randomness_user::uses_randomness_indirect();
    }
}
