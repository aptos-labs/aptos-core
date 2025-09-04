# Summary

PVSS schemes:

 - Das PVSS with PKs in $\mathbb{G}_2$ and SKs in $\mathbb{G}_1$

WVUF schemes:
 - Pinkas WVUF with PKs in $\mathbb{G}_2$ and SKs in $\mathbb{G}_1$

# TODOs

 - `accumulator_poly` uses a hard-coded FFT threshold to decide when to switch between slow/fast implementations
 - multi-threaded `accumulator_poly`
 - eliminate extra allocations in `accumulator_poly`
 - Consider using affine coordinates for the in-memory representation of a transcript/VUF as a way to speed up multiexps / multipairings

# Workarounds

## rand_core_hell

`velor_crypto` uses `rand_core 0.5.1`. However, `blstrs` uses `rand_core 0.6`

This spells disaster: we cannot pass the RNGs from `rand_core 0.6` into the `velor_crypto` traits that expect a `0.5.1`-version RNG.

We work around this by generating random seeds using the 0.5.1 RNG that we get from `velor_crypto` and then create `Scalar`s and points from those seeds manually by hashing the seed to a curve point.

## blstrs quirks

### Size-1 multiexps

`blstrs 0.7.0` had a bug (originally from `blst`) where size-1 multiexps (sometimes) don't output the correct result: see [this issue](https://github.com/filecoin-project/blstrs/issues/57) opened by Sourav Das.

As a result, some of our 1 out of 1 weighted PVSS tests which did a secret reconstruction via a size-1 multiexp in G2 failed intermittently. (This test was called `weighted_fail` at commit `5cd69cba8908b6676cf4481457aae93850b6245e`; it runs in a loop until it fails; sometimes it doesn't fail; most of the times it does though.)

We patched this by clumsily checking for the input size before calling `blstrs`'s multiexp wrapper.

### $g_1^0$ and $g_2^0$ multiexps can fail
test_crypto_g1_multiexp_less_points
See `test_crypto_g_2_to_zero_multiexp` and `test_crypto_g_1_to_zero_multiexp`.

### Multiexps with more exponents than bases fail. 

See `test_crypto_g1_multiexp_less_points`.

Instead, they should truncate the exponents to be the size of the bases.

### Support for generics is lacking

e.g., no multiexp trait on G1, G2 and GT

### Cannot sample GT from random bytes due to unexposed `fp12::Fp12`

# Notes

We (mostly) rely on the `velor-crypto` `SerializeKey` and `DeserializeKey` derives for safety during deserialization.
Specifically, each cryptographic object (e.g., public key, public parameters, etc) must implement `ValidCryptoMaterial` for serialization and `TryFrom` for deserialization when these derives are used.

The G1/G2 group elements in `blstrs` are deserialized safely via calls to `from_[un]compressed` rather than calls to `from_[un]compressed_unchecked` which does not check prime-order subgroup membership.

Our structs use $(x, y, z)$ projective coordinates, for faster arithmetic operations.
During serialization, we convert to more succinct $(x, y)$ affine coordinates.

# Cargo flamegraphs

Example: You indicate the benchmark group with `--bench` and then you append part of the benchmark name at the end (e.g., `accumulator_poly/` so as to exclude `accumulator_poly_slow/`)
```
sudo cargo flamegraph --bench 'crypto' -- --bench accumulator_poly/
```
# References

[GJM+21e] Aggregatable Distributed Key Generation; by Kobi Gurkan and Philipp Jovanovic and Mary Maller and Sarah Meiklejohn and Gilad Stern and Alin Tomescu; in Cryptology ePrint Archive, Report 2021/005; 2021; https://eprint.iacr.org/2021/005
