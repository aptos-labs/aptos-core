spec std::bit_vector {

    // Makes `length` opaque so callers use the spec rather than inlining the body.
    // The ensures is equivalent to the source impl but stated in terms of the field.
    spec length(self: &BitVector): u64 {
        pragma opaque = true;
        ensures [inferred] self.length == len(self.bit_field) ==> result == len(self.bit_field);
        aborts_if [inferred] false;
    }

}
