spec std::result {

    spec is_err<T, E>(self: &0x1::result::Result<T, E>): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is Err);
        aborts_if [inferred] false;
    }

    spec is_ok<T, E>(self: &0x1::result::Result<T, E>): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is Ok);
        aborts_if [inferred] false;
    }

    spec unwrap<T, E>(self: 0x1::result::Result<T, E>): T {
        pragma opaque = true;
        ensures [inferred](self is Ok) ==> result == self.Ok.0;
        aborts_if [inferred] self is Err;
    }

    spec unwrap_err<T, E>(self: 0x1::result::Result<T, E>): E {
        pragma opaque = true;
        ensures [inferred](self is Err) ==> result == self.Err.0;
        aborts_if [inferred] self is Ok;
    }
}
