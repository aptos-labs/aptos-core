# $\Sigma$-protocols README

## Documentation

The Markdown documents are already available in [../../../doc](../../../doc/).
You can view them with your favorite Markdown viewer/editor.
Or, you can compile the Markdown to HTML and view them in a browser.

```bash
cd aptos-move/framework/aptos-experimental/doc
pandoc sigma_protocol_key_rotation.md -s --mathjax -o sigma_protocol_key_rotation.html
```

If the Markdown is out of date (e.g., because you've made changes to the Move code), you can regenarate it via:
```
cargo build -p aptos-cached-packages`
```
Or, much faster via:
```bash
cd aptos-move/framework/aptos-experimental/sources/confidential_asset
aptos move document --include-impl --collapsed-sections
```

## Implementing $\Sigma$-protocols securely in Move using this library

Due to Move's limitations (e.g., lack of traits), our $\Sigma$-protocol library is rather primitive and potentially-dangerous if not used carefully.

When implementing a $\Sigma$-protocol for you relation, correctly implementing its homomorphism $\psi$ and transformation function $f$ in Move can be tricky. If done wrong, soundness and privacy will be lost $$\Rightarrow$$ your protocol will likely be insecure.

Here are a few guiding principles to save you.

**First**, be sure to correctly build the `RepresentationVec` describing all the MSMs that the homomorphism $\psi$ performs.
**Second**, it is crucial to check the following sizes are correct:

 1. \# of points in public statement is correct
 2. \# of scalars in public statement is correct
 3. \# of scalars in secret witness is correct
 4. \# of points in homomorphism's output is correct

The same principles apply when implementing the transformation function $f$!
There, usually, the `RepresentationVec` being built is much simpler and thus less error-prone.
In an abundance of caution, we will recommend redundantly re-performing the four size checks above in $f$'s implementation too.

One important clarification is that both $\psi$'s and $f$'s implementations in Move never need to perform operations directly with the elliptic curve points stored in the `Statement`, but only with "pointers" to them: i.e., their indices in the `Statement::points` vector is used to build `Representation`'s describing the MSMs that the homomorphism computes.
In other words, the implementations of $\psi$ and $f$ return "representations" (a `RepresentationVec`), not `ristretto255::RistrettoPoint`'s.

**Note:** I am not yet sure if there may be cases where the homomorphism would need to work directly with the scalars in the `Statement`.
There are certainly cases where the transformation function needs to work directly with the scalars (e.g., the public withdrawal NP relation).

The Schnorr and PedEq examples in the [../../../tests/confidential_asset/sigma_protocols/](../../../tests/confidential_asset/sigma_protocols/) directory should serve as good examples of how to do this right.
