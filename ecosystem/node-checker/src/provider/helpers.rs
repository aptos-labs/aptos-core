// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub const MISSING_PROVIDER_MESSAGE: &str = "Incomplete request";

/// This macro helps you turn an Option<P> where P is a Provider or Arc<Provider>
/// into a &P, returning a CheckResult if the Option is None and required is true,
/// or just an empty vec if required is false.
///
/// Example invocations:
///
/// ```
/// # use std::io;
/// let target_metrics_provider = aptos_node_checker_lib::get_provider!(
///     input.target_metrics_provider,
///     true,
///     MetricsProvider
/// )?;
/// # Ok::<_, io::Error>(vec![])
/// ```
///
/// The first argument is the Option, the second is `required`, and the last is
/// the type of the Provider, which is used for building error messages.
///
/// This macro must be used within the context of a Checker, it will not work
/// anywhere else.
#[macro_export]
macro_rules! get_provider {
    ($provider_option:expr, $required:expr, $provider_type:ty) => {
        match $provider_option {
            Some(ref provider) => provider,
            None => {
                if $required {
                    let checker_type_name = $crate::common::get_type_name::<Self>();
                    let provider_type_name = $crate::common::get_type_name::<$provider_type>();
                    return Ok(vec![CheckResult::new(
                        // This line is why this macro will only work inside a Checker.
                        checker_type_name.to_string(),
                        format!("{}: {}", checker_type_name, $crate::provider::MISSING_PROVIDER_MESSAGE),
                        0,
                        format!(
                            "Failed to fetch the data for the {} because of an error originating from the {}: {}",
                            checker_type_name,
                            provider_type_name,
                            <$provider_type as $crate::provider::Provider>::explanation()
                        ),
                    )]);
                } else {
                    return Ok(vec![]);
                }
            }
        }
    };
}
