# "Dude, is my code constant-time?" checks

Run from the `crates/aptos-crypto` directory via:
```
cargo run --release --example is_zkcrypto_constant_time
cargo run --release --example is_blstrs_constant_time
```

Quoting from the original README [here](https://docs.rs/dudect-bencher/latest/dudect_bencher/):
The program output will look like:

```
bench constant_time::blstrs_scalar_mul::run_bench seeded with 0xc41137dfdf1b2629
bench constant_time::blstrs_scalar_mul::run_bench ...
# of 1 bits in scalars for "left" class is in [1, 4)
# of 1 bits in scalars for "right" class is always 250
: n == +0.010M, max t = +1.86799, max tau = +0.01869, (5/tau)^2 = 71538
```

It is interpreted as follows.
Firstly, note that the runtime distributions are cropped at different percentiles and about 100 t-tests are performed.
Of these t-tests, the one that produces the largest absolute t-value is printed as max_t. 
The other values printed are:

 - `n`, indicating the number of samples used in computing this t-value
 - `max_tau`, which is the t-value scaled for the samples size (formally, `max_tau = max_t / sqrt(n)`)
 - `(5/tau)^2`, which indicates the number of measurements that would be needed to distinguish the two distributions with `t > 5`

t-values outside (-5, 5) (i.e., of absolute value > 5) are generally considered a good indication that the function is not constant time. 
t-values in (-5, 5) does not necessarily imply that the function is constant-time, since there may be other input distributions under which the function behaves significantly differently.
