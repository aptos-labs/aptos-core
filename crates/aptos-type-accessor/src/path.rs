// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use move_core_types::identifier::Identifier;
use std::str::FromStr;

/// Parse a path into a vector of [`PathComponent`]s.
///
/// For example, the path:
/// ```
/// "fruit_inventory.1.color.red";
/// # ()
/// ```
/// would be parsed into:
/// ```
/// # use aptos_type_accessor::path::PathComponent;
/// # use move_core_types::identifier::Identifier;
/// #
/// [
///     PathComponent::Field(Identifier::new("fruit_inventory")?),
///     PathComponent::GenericTypeParamIndex(1),
///     PathComponent::Field(Identifier::new("color")?),
///     PathComponent::Field(Identifier::new("red")?),
/// ];
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// Similarly, the path:
/// ```
/// "authorized_buyers.[].address";
/// # ()
/// ```
/// would be parsed into:
/// ```
/// # use aptos_type_accessor::path::PathComponent;
/// # use move_core_types::identifier::Identifier;
/// #
/// [
///     PathComponent::Field(Identifier::new("authorized_buyers")?),
///     PathComponent::EnterArray,
///     PathComponent::Field(Identifier::new("address")?),
/// ];
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// This syntax exploits known restrictions on Move field names. Making use of
/// `.`, `.<number>`, and `.[]` is safe because, in the case of `.` and `.[]`,
/// Move field names can't contain those characters, and in the case of
/// `.<number>`, Move field names can't start with a number.
pub fn parse_path(path: &str) -> Result<Vec<PathComponent>> {
    path.split('.').map(PathComponent::from_str).collect()
}

/// A single component of a path specifier to be used with [`crate::TypeAccessor`].
/// See more in the documentation for [`parse_path`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathComponent {
    /// Access a field of a struct.
    Field(Identifier),
    /// Access a generic type parameter of a type.
    GenericTypeParamIndex(u16),
    /// Access the type inside an array.
    EnterArray,
}

impl FromStr for PathComponent {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(index) = s.parse::<u16>() {
            Ok(PathComponent::GenericTypeParamIndex(index))
        } else if s == "[]" {
            Ok(PathComponent::EnterArray)
        } else {
            Ok(PathComponent::Field(Identifier::new(s).with_context(
                || format!("String {} is not a valid identifier", s),
            )?))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_path() {
        assert_eq!(parse_path("fruit_inventory.1.color.red").unwrap(), vec![
            PathComponent::Field(Identifier::new("fruit_inventory").unwrap()),
            PathComponent::GenericTypeParamIndex(1),
            PathComponent::Field(Identifier::new("color").unwrap()),
            PathComponent::Field(Identifier::new("red").unwrap()),
        ]);
        assert_eq!(parse_path("authorized_buyers.[].address").unwrap(), vec![
            PathComponent::Field(Identifier::new("authorized_buyers").unwrap()),
            PathComponent::EnterArray,
            PathComponent::Field(Identifier::new("address").unwrap()),
        ]);
    }
}
