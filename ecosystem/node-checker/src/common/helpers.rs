// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// Same thing as std::any::type_name but it returns only the final section
/// after the last "::".
pub fn get_type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .split("::")
        .last()
        .unwrap_or("UnknownType")
}
