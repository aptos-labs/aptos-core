This crate contains a single test for CI to check whether someone has pulled
a build dependency on `cached-packages` and therefore generated
artifacts are up-to-date. The test must be standalone in its
own crate to ensure running it does not itself request build of
`cached-packages`.
