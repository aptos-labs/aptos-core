# diem-workspace-hack

This crate is hack to unify 3rd party dependency crate features in order to
have better compilation caching. Each time cargo is invoked, it computes
features, so across multi invocations it may resolve differently and therefore
find things are cached.

The downside is that all code is compiled with the features needed for any
invocation.

See the
[cargo-hakari documentation](https://docs.rs/cargo-hakari/latest/cargo_hakari/)
for further details.
