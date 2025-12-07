// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Same thing as std::any::type_name but it returns only the final section
/// after the last "::".
pub fn get_type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .split("::")
        .last()
        .unwrap_or("UnknownType")
}
