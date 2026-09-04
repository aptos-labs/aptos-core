// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, AttributeArgs, FnArg, Ident, ItemFn, Lit, Meta, NestedMeta, Pat, ReturnType,
};

/// Runs a test twice in the same process: once on the V1 VM, then once with the
/// MonoMove VM. The MonoMove pass only runs if the V1 pass passed, and its
/// failures are reported as MonoMove failures.
///
/// Must be placed above `#[test]`:
///
/// ```ignore
/// #[run_mono_move]
/// #[test]
/// fn my_test() { .. }
/// ```
///
/// It also goes above `#[test_case]` or `#[rstest]`, in which case the test
/// takes arguments and each pass gets its own clone of them:
///
/// ```ignore
/// #[run_mono_move]
/// #[test_case(true)]
/// #[test_case(false)]
/// fn my_test(enabled: bool) { .. }
/// ```
///
/// Placing it *below* `#[test_case]` does not work: that macro strips the
/// attribute off the original function and re-attaches it under the `#[test]`
/// it generates.
///
/// A test known to fail under MonoMove can be marked
/// `#[run_mono_move(should_fail = "why")]`. The reason is documentation; it is
/// not matched against the failure. A test that would *hang* under MonoMove --
/// anything relying on gas exhaustion to terminate -- must not be annotated at
/// all.
#[proc_macro_attribute]
pub fn run_mono_move(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(item as ItemFn);

    let should_fail = match parse_should_fail(args) {
        Ok(should_fail) => should_fail,
        Err(err) => return err,
    };
    if let Err(err) = check_signature(&input) {
        return err;
    }
    let params = match parameter_names(&input) {
        Ok(params) => params,
        Err(err) => return err,
    };

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
        ..
    } = input;
    let name = &sig.ident;
    let name_str = name.to_string();
    let inputs = &sig.inputs;
    let should_fail = match should_fail {
        Some(reason) => quote! { ::std::option::Option::Some(#reason) },
        None => quote! { ::std::option::Option::None },
    };

    // The body is hoisted into an item rather than being inlined into the
    // closure, so that it cannot capture anything but the parameters. Each pass
    // gets a fresh clone of those, so the MonoMove pass never sees a mutation
    // the V1 pass made.
    quote! {
        #(#attrs)*
        #vis fn #name(#inputs) {
            fn body(#inputs) #block
            ::aptos_move_e2e_test_harness::mono_move_test::run(
                #name_str,
                #should_fail,
                &|| body(#(::std::clone::Clone::clone(&#params)),*),
            );
        }
    }
    .into()
}

fn parse_should_fail(args: AttributeArgs) -> Result<Option<String>, TokenStream> {
    let mut should_fail = None;
    for arg in args {
        let name_value = match arg {
            NestedMeta::Meta(Meta::NameValue(name_value))
                if name_value.path.is_ident("should_fail") =>
            {
                name_value
            },
            _ => return Err(error(r#"expected `should_fail = "reason"`"#)),
        };
        match name_value.lit {
            Lit::Str(reason) => should_fail = Some(reason.value()),
            _ => return Err(error("`should_fail` expects a string literal")),
        }
    }
    Ok(should_fail)
}

fn check_signature(input: &ItemFn) -> Result<(), TokenStream> {
    let sig = &input.sig;
    // `#[test]` expands before any attribute placed below it, so its absence on
    // a test taking no arguments means the two were written the wrong way
    // round. Arguments instead mean a `#[test_case]`/`#[rstest]` harness, which
    // supplies the `#[test]` on the functions it generates.
    if sig.inputs.is_empty() && !input.attrs.iter().any(|attr| attr.path.is_ident("test")) {
        return Err(error(
            "`#[run_mono_move]` must be placed above `#[test]`, `#[test_case]` or `#[rstest]`",
        ));
    }
    if sig.asyncness.is_some()
        || !sig.generics.params.is_empty()
        || !matches!(sig.output, ReturnType::Default)
    {
        return Err(error(
            "`#[run_mono_move]` only supports a plain test: no generics, not \
             async, returning `()`",
        ));
    }
    Ok(())
}

/// The parameter names, which must all be plain identifiers so that the
/// generated closure can pass a clone of each one through to the body.
fn parameter_names(input: &ItemFn) -> Result<Vec<Ident>, TokenStream> {
    input
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
                Pat::Ident(pat_ident) if pat_ident.subpat.is_none() => Ok(pat_ident.ident.clone()),
                _ => Err(error(
                    "`#[run_mono_move]` requires every test parameter to be a plain identifier",
                )),
            },
            FnArg::Receiver(_) => Err(error("`#[run_mono_move]` cannot be used on a method")),
        })
        .collect()
}

fn error(msg: &str) -> TokenStream {
    syn::Error::new(Span::call_site(), msg)
        .to_compile_error()
        .into()
}
