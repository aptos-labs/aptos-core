// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ability::AbilitySet,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use serde::{de::Error, ser::SerializeSeq, Deserialize, Serialize};
use std::fmt;

/// Version number for the serialization format of function data.
pub const FUNCTION_DATA_SERIALIZATION_FORMAT_V1: u16 = 1;

//===========================================================================================

/// A `ClosureMask` is a value which determines how to distinguish those function arguments
/// which are captured and which are not when a closure is constructed. For instance,
/// with `_` representing an omitted argument, the mask for `f(a,_,b,_)` would have the argument
/// at index 0 and at index 2 captured. The mask can be used to transform lists of types.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary),
    derive(dearbitrary::Dearbitrary)
)]
pub struct ClosureMask(u64);

impl fmt::Display for ClosureMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl ClosureMask {
    /// The maximal number of arguments which can be handled by a closure mask.
    /// A captured argument's position in the argument list must be lower than
    /// this number. Notice that this property is implicit in the bytecode:
    /// a PACK_CLOSURE instruction will never pop more arguments from the
    /// stack than this number.
    pub const MAX_ARGS: usize = 64;

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn new(mask: u64) -> Self {
        Self(mask)
    }

    pub fn new_for_leading(n: usize) -> Result<Self, String> {
        let mut mask = Self::new(0);
        for i in 0..n {
            mask.set_captured(i)?;
        }
        Ok(mask)
    }

    pub fn bits(&self) -> u64 {
        self.0
    }

    /// Returns true if the i'th argument is captured. If `i` is out of range, false will
    /// be returned.
    pub fn is_captured(&self, i: usize) -> bool {
        i < Self::MAX_ARGS && self.0 & (1 << i) != 0
    }

    /// Sets the ith argument to be captured
    pub fn set_captured(&mut self, i: usize) -> Result<(), String> {
        if i >= Self::MAX_ARGS {
            return Err(format!(
                "Argument index {} exceeds maximum allowed arguments {}",
                i,
                Self::MAX_ARGS
            ));
        }
        self.0 |= 1 << i;
        Ok(())
    }

    /// Apply a closure mask to a list of elements, returning only those
    /// where position `i` is set in the mask (if `collect_captured` is true) or not
    /// set (otherwise).
    pub fn extract<'a, T>(
        &self,
        values: impl IntoIterator<Item = &'a T>,
        collect_captured: bool,
    ) -> Vec<&'a T> {
        let mut mask = self.0;
        values
            .into_iter()
            .filter(|_| {
                let set = mask & 0x1 != 0;
                mask >>= 1;
                set && collect_captured || !set && !collect_captured
            })
            .collect()
    }

    /// Compose two lists of elements into one based on the given mask such that the
    /// following holds:
    /// ```ignore
    ///   mask.compose(mask.extract(v, true), mask.extract(v, false)) == v
    /// ```
    /// This returns `None` if the provided lists are inconsistent w.r.t the mask
    /// and cannot be composed. This should not happen in verified code, but
    /// a caller should decide whether to crash or to error.
    pub fn compose<T>(
        &self,
        captured: impl IntoIterator<Item = T>,
        provided: impl IntoIterator<Item = T>,
    ) -> Option<Vec<T>> {
        let mut captured = captured.into_iter();
        let mut provided = provided.into_iter();
        let mut result = vec![];
        let mut mask = self.0;
        while mask != 0 {
            if mask & 0x1 != 0 {
                result.push(captured.next()?)
            } else {
                result.push(provided.next()?)
            }
            mask >>= 1;
        }
        if captured.next().is_some() {
            // Not all captured arguments consumed
            return None;
        }
        result.extend(provided);
        Some(result)
    }

    /// Return the max index of captured argument, or None if none is captured.
    pub fn max_captured(&self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let mut i = 0;
        let mut mask = self.0;
        loop {
            mask >>= 1;
            if mask == 0 {
                return Some(i);
            }
            i += 1
        }
    }

    /// Return the # of captured arguments in the mask
    pub fn captured_count(&self) -> u16 {
        let mut i = 0;
        let mut mask = self.0;
        while mask != 0 {
            if mask & 0x1 != 0 {
                i += 1
            }
            mask >>= 1;
        }
        i
    }

    /// Given a vector of captured arguments (converted into strings), formats them as a vector of
    /// all arguments such that:
    ///   - if argument is captured, an entry from the vector is added to the final vector,
    ///   - if argument is not captured, "_" is added to the final vector.
    /// The last element of a vector is "..", indicating possibly mor non-captured arguments (it is
    /// not possible to deduce if there are any because the mask is simply 0).
    ///
    /// In case there is any error, a vector of a single dummy value is returned.
    pub fn format_arguments(&self, captured: Vec<String>) -> Vec<String> {
        // If the function returns None, this means not all arguments were captured. Should not
        // happen. Do not return an error because this is used to implement `Display`, which can
        // make `format!` panic.
        self.format_arguments_impl(captured)
            .unwrap_or_else(|| vec!["*invalid*".to_string()])
    }

    fn format_arguments_impl(&self, captured: Vec<String>) -> Option<Vec<String>> {
        let mut mask = self.0;
        let mut captured = captured.into_iter();

        let mut result = vec![];
        while mask != 0 {
            if mask & 0x1 != 0 {
                result.push(captured.next()?)
            } else {
                result.push("_".to_string())
            }
            mask >>= 1;
        }

        // We do not know arity information of the function, so the simplest option is to indicate
        // that there can be more arguments in the end.
        result.push("..".to_string());

        if captured.next().is_some() {
            return None;
        }

        Some(result)
    }
}

#[cfg(test)]
mod closure_mask_tests {
    use super::*;

    #[test]
    fn extract_compose_roundtrip_test() {
        // mask.compose(mask.extract(v, true), mask.extract(v, false)) == v
        let mask = ClosureMask::new(0b101011);
        let v = vec![1, 2, 3, 4, 5, 6];
        let captured = mask.extract(&v, true);
        assert_eq!(captured, vec![&1, &2, &4, &6]);
        let not_captured = mask.extract(&v, false);
        assert_eq!(not_captured, vec![&3, &5]);
        assert_eq!(
            v,
            mask.compose(captured, not_captured)
                .expect("composition must succeed")
                .into_iter()
                .copied()
                .collect::<Vec<_>>()
        );
    }
}

//===========================================================================================

/// Function type layout, with arguments and result types.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveFunctionLayout(
    pub Vec<MoveTypeLayout>,
    pub Vec<MoveTypeLayout>,
    pub AbilitySet,
);

/// A closure (function value). The closure stores the name of the function and it's
/// type instantiation, as well as the closure mask and the captured values together
/// with their layout. The latter allows to deserialize closures context free (without
/// needing to lookup information about the function and its dependencies).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveClosure {
    pub module_id: ModuleId,
    pub fun_id: Identifier,
    pub ty_args: Vec<TypeTag>,
    pub mask: ClosureMask,
    pub captured: Vec<(MoveTypeLayout, MoveValue)>,
}

pub(crate) struct ClosureVisitor;

impl<'d> serde::de::Visitor<'d> for ClosureVisitor {
    type Value = MoveClosure;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Closure")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let version = read_required_value::<_, u16>(&mut seq)?;
        if version != FUNCTION_DATA_SERIALIZATION_FORMAT_V1 {
            return Err(A::Error::custom(format!(
                "unexpected function data version {}",
                version
            )));
        }
        let module_id = read_required_value::<_, ModuleId>(&mut seq)?;
        let fun_id = read_required_value::<_, Identifier>(&mut seq)?;
        let ty_args = read_required_value::<_, Vec<TypeTag>>(&mut seq)?;
        let mask = read_required_value::<_, ClosureMask>(&mut seq)?;
        let mut captured = vec![];
        for _ in 0..mask.captured_count() {
            let layout = read_required_value::<_, MoveTypeLayout>(&mut seq)?;
            match seq.next_element_seed(&layout)? {
                Some(v) => captured.push((layout, v)),
                None => return Err(A::Error::invalid_length(captured.len(), &self)),
            }
        }
        // If the sequence length is known, check whether there are no extra values
        if matches!(seq.size_hint(), Some(remaining) if remaining != 0) {
            return Err(A::Error::invalid_length(captured.len(), &self));
        }
        Ok(MoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
        })
    }
}

fn read_required_value<'a, A, T>(seq: &mut A) -> Result<T, A::Error>
where
    A: serde::de::SeqAccess<'a>,
    T: serde::de::Deserialize<'a>,
{
    match seq.next_element::<T>()? {
        Some(x) => Ok(x),
        None => Err(A::Error::custom("expected more elements")),
    }
}

impl serde::Serialize for MoveClosure {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let MoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
        } = self;
        let mut s = serializer.serialize_seq(Some(5 + captured.len() * 2))?;
        s.serialize_element(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1)?;
        s.serialize_element(module_id)?;
        s.serialize_element(fun_id)?;
        s.serialize_element(ty_args)?;
        s.serialize_element(mask)?;
        for (l, v) in captured {
            s.serialize_element(l)?;
            s.serialize_element(v)?;
        }
        s.end()
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use crate::{
        account_address::AccountAddress,
        ident_str,
        value::{MoveStruct, MoveStructLayout},
    };

    #[test]
    fn function_value_serialization_ok() {
        let value = MoveValue::Closure(Box::new(MoveClosure {
            module_id: ModuleId {
                address: AccountAddress::ONE,
                name: ident_str!("mod").to_owned(),
            },
            fun_id: ident_str!("func").to_owned(),
            ty_args: vec![TypeTag::Bool],
            mask: ClosureMask::new(0b111),
            captured: vec![
                (MoveTypeLayout::U64, MoveValue::U64(2066)),
                (
                    MoveTypeLayout::Vector(Box::new(MoveTypeLayout::Bool)),
                    MoveValue::Vector(vec![MoveValue::Bool(false)]),
                ),
                (
                    MoveTypeLayout::Struct(MoveStructLayout::Runtime(vec![
                        MoveTypeLayout::Bool,
                        MoveTypeLayout::U8,
                    ])),
                    MoveValue::Struct(MoveStruct::Runtime(vec![
                        MoveValue::Bool(false),
                        MoveValue::U8(22),
                    ])),
                ),
            ],
        }));
        let blob = value
            .simple_serialize()
            .expect("serialization must succeed");
        eprintln!("{:?}", blob);
        assert_eq!(
            value,
            MoveValue::simple_deserialize(&blob, &MoveTypeLayout::Function)
                .expect("deserialization must succeed"),
            "deserialized value not equal to original one"
        );
    }
}
