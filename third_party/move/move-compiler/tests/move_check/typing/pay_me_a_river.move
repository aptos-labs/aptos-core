module 0x8675309::M {
    struct Payments has key, drop  {
	value: u64
    }

    public fun create_stream(asigner: signer): u64
    {
        let payments: &mut Payments;
        payments = &mut Payments {
	    // let payments: &mut Payments = Payments {
	    value: 3
        };
        move_to(&asigner, payments);
	payments.value
    }
}
