/// Provides the `Result<T, E>` type, which allows to represent a success value `T` or an error value `E`.
module std::result {
    use std::error;

    /// Attempt to unwrap value but found error
    const E_UNWRAP_OK: u64 = 0;

    /// Attempt to unwrap error but found value
    const E_UNWRAP_ERR: u64 = 1;

    /// Represents the result of some computation, either a value `T` or an error `E`.
    enum Result<T, E> has copy, store {
        Ok(T),
        Err(E)
    }

    /// Checks whether the result is Ok.
    public fun is_ok<T, E>(self: &Result<T, E>): bool {
        self is Ok
    }

    /// Checks whether the result is Err.
    public fun is_err<T, E>(self: &Result<T, E>): bool {
        self is Err
    }

    /// Unpacks the `T` of Ok or aborts.
    public fun unwrap<T, E>(self: Result<T, E>): T {
        match (self) {
            Ok(x) => x,
            _ => abort error::invalid_argument(E_UNWRAP_OK)
        }
    }

    /// Unpacks the `E` of Err or aborts.
    public fun unwrap_err<T, E>(self: Result<T, E>): E {
        match (self) {
            Err(x) => x,
            _ => abort error::invalid_argument(E_UNWRAP_ERR)
        }
    }

    // TODO: add below functions once language version 2.4 is enabled
    /*
    /// Maps a `T` if it is available.
    public inline fun map<T, E, TNew>(self: Result<T, E>, f: |T|TNew): Result<TNew, E> {
        match (self) {
            Ok(x) => Result::Ok(f(x)),
            Err(e) => Result::Err(e)
        }
    }

    /// Maps a `E` if it is available.
    public inline fun map_err<T, E, ENew>(self: Result<T, E>, f: |E|ENew): Result<T, ENew> {
        match (self) {
            Ok(x) => Result::Ok(x),
            Err(e) => Result::Err(f(e))
        }
    }

    /// Continues a `T` if it is available.
    public inline fun and_then<T, E, TNew>(self: Result<T, E>, f: |T|Result<TNew, E>): Result<TNew, E> {
        match (self) {
            Ok(x) => f(x),
            Err(e) => Result::Err(e)
        }
    }

    /// Tries an alternative if not Ok.
    public inline fun or_else<T, E>(self: Result<T, E>, f: |E|Result<T, E>): Result<T, E> {
        match (self) {
            Err(e) => f(e),
            ok => ok
        }
    }
    */
}
