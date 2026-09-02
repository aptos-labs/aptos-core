spec std::signer {

    spec address_of(self: &signer): address {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] result == borrow_address(self);
        aborts_if false;
    }
}
