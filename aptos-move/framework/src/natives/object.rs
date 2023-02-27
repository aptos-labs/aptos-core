// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::Op,
    gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    language_storage::TypeTag,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    native_functions::{NativeContext, NativeFunction},
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{GlobalValue, Value},
};
use smallvec::smallvec;
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
    fmt::Display,
    sync::Arc,
};
use move_core_types::language_storage::StructTag;


// ===========================================================================================
// Public Data Structures and Constants

#[derive(Clone, Debug)]
pub struct ObjectType(pub TypeTag);

impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Object<{}>", self.0)
    }
}

/// An object change set.
#[derive(Default)]
pub struct ObjectChangeSet {
    pub new_objects: BTreeSet<AccountAddress>,
    pub removed_objects: BTreeSet<AccountAddress>,
    pub changes: BTreeMap<AccountAddress, ObjectChange>,
}

/// A change of a single Object.
pub struct ObjectChange {
    pub entries: BTreeMap<StructTag, Op<Vec<u8>>>,
}

/// An object resolver which needs to be provided by the environment. This allows to lookup
/// data in remote storage, as well as retrieve cost of object operations.
pub trait ObjectResolver {
    fn resolve_object_entry(
        &self,
        id: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, anyhow::Error>;
}

/// The native object context extension. This needs to be attached to the NativeContextExtensions
/// value which is passed into session functions, so its accessible from natives of this
/// extension.
#[derive(Tid)]
pub struct NativeObjectContext<'a> {
    resolver: &'a dyn ObjectResolver,
    object_data: RefCell<ObjectData>,
}

// See stdlib/Error.move
const _ECATEGORY_INVALID_STATE: u8 = 0;
const ECATEGORY_INVALID_ARGUMENT: u8 = 7;

const ALREADY_EXISTS: u64 = (100 << 8) + ECATEGORY_INVALID_ARGUMENT as u64;
const NOT_FOUND: u64 = (101 << 8) + ECATEGORY_INVALID_ARGUMENT as u64;
// Move side raises this
const _NOT_EMPTY: u64 = (102 << 8) + _ECATEGORY_INVALID_STATE as u64;

// ===========================================================================================
// Private Data Structures and Constants

/// A structure representing muobject data of the NativeObjectContext. This is in a RefCell
/// of the overall context so we can mutate while still accessing the overall context.
#[derive(Default)]
struct ObjectData {
    new_objects: BTreeSet<AccountAddress>,
    removed_objects: BTreeSet<AccountAddress>,
    objects: BTreeMap<AccountAddress, Object>,
}

/// A structure representing a single object.
struct Object {
    id: AccountAddress,
    content: BTreeMap<StructTag, (MoveTypeLayout,  GlobalValue)>,
}

// =========================================================================================
// Implementation of Native Object Context

impl<'a> NativeObjectContext<'a> {
    /// Create a new instance of a native object context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(resolver: &'a dyn ObjectResolver) -> Self {
        Self {
            resolver,
            object_data: Default::default(),
        }
    }

    /// Computes the change set from a NativeObjectContext.
    pub fn into_change_set(self) -> PartialVMResult<ObjectChangeSet> {
        let NativeObjectContext { object_data, .. } = self;
        let ObjectData {
            new_objects,
            removed_objects,
            objects,
        } = object_data.into_inner();
        let mut changes = BTreeMap::new();
        for (id, object) in objects {
            let Object {
                content,
                ..
            } = object;
            let mut entries = BTreeMap::new();
            for (key, (value_layout, gv)) in content {
                let op = match gv.into_effect() {
                    Some(op) => op,
                    None => continue,
                };

                match op {
                    Op::New(val) => {
                        let bytes = serialize(&value_layout, &val)?;
                        entries.insert(key, Op::New(bytes));
                    }
                    Op::Modify(val) => {
                        let bytes = serialize(&value_layout, &val)?;
                        entries.insert(key, Op::Modify(bytes));
                    }
                    Op::Delete => {
                        entries.insert(key, Op::Delete);
                    }
                }
            }
            if !entries.is_empty() {
                changes.insert(id, ObjectChange { entries });
            }
        }
        Ok(ObjectChangeSet {
            new_objects,
            removed_objects,
            changes,
        })
    }
}

impl ObjectData {
    /// Gets or creates a new object in the ObjectData. This initializes information about
    /// the object, like the type layout for keys and values.
    fn get_or_create_object(
        &mut self,
        id: AccountAddress,
    ) -> PartialVMResult<&mut Object> {
        Ok(match self.objects.entry(id) {
            Entry::Vacant(e) => {
                let object = Object {
                    id,
                    content: Default::default(),
                };
                e.insert(object)
            }
            Entry::Occupied(e) => e.into_mut(),
        })
    }
}

impl Object {
    fn get_or_create_global_value(
        &mut self,
        context: &NativeContext,
        key: StructTag,
        value_layout: MoveTypeLayout,
    ) -> PartialVMResult<(&mut GlobalValue, Option<Option<NumBytes>>)> {
        Ok(match self.content.entry(key) {
            Entry::Vacant(entry) => {
                let (layout_and_gv, loaded) = match context
                    .extensions().get::<NativeObjectContext>()
                    .resolver
                    .resolve_object_entry(&self.id, entry.key())
                    .map_err(|err| {
                        partial_extension_error(format!("remote object resolver failure: {}", err))
                    })? {
                    Some(val_bytes) => {
                        let val = deserialize(&value_layout, &val_bytes)?;
                        (
                            (value_layout, GlobalValue::cached(val)?),
                            Some(NumBytes::new(val_bytes.len() as u64)),
                        )
                    }
                    None => ((value_layout, GlobalValue::none()), None),
                };
                (entry.insert(layout_and_gv), Some(loaded))
            }
            Entry::Occupied(entry) => (entry.into_mut(), None),
        }).map(|((_layout, v), bytes)| (v, bytes))
    }
}
#[derive(Debug, Clone)]
pub struct CommonGasParameters {
    pub load_base: InternalGas,
    pub load_per_byte: InternalGasPerByte,
    pub load_failure: InternalGas,
}

impl CommonGasParameters {
    fn calculate_load_cost(&self, loaded: Option<Option<NumBytes>>) -> InternalGas {
        self.load_base
            + match loaded {
            Some(Some(num_bytes)) => self.load_per_byte * num_bytes,
            Some(None) => self.load_failure,
            None => 0.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
}

fn native_add(
    common_gas_params: &CommonGasParameters,
    gas_params: &AddGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 1);
    assert_eq!(args.len(), 2);

    let object_context = context.extensions().get::<NativeObjectContext>();
    let mut object_data = object_context.object_data.borrow_mut();

    let val = args.pop_back().unwrap();
    let address = pop_arg!(args, AccountAddress);
    let object = object_data.get_or_create_object(address)?;

    let mut cost = gas_params.base;
    let key = ty_args.pop().unwrap();
    let struct_tag = match context.type_to_type_tag(&key)? {
        TypeTag::Struct(struct_tag) => *struct_tag,
        _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
    };
    // let key_bytes = bcs::to_bytes(&context.type_to_type_tag(&key)?).map_err(|_| partial_extension_error("cannot serialize object type tag"))?;
    // cost += gas_params.per_byte_serialized * NumBytes::new(key_bytes.len() as u64);

    let value_layout = get_type_layout(context, &key)?;
    let (gv, loaded) = object.get_or_create_global_value(context, struct_tag, value_layout)?;
    cost += common_gas_params.calculate_load_cost(loaded);

    match gv.move_to(val) {
        Ok(_) => Ok(NativeResult::ok(cost, smallvec![])),
        Err(_) => Ok(NativeResult::err(cost, ALREADY_EXISTS)),
    }
}

pub fn make_native_add(
    common_gas_params: CommonGasParameters,
    gas_params: AddGasParameters,
) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_add(&common_gas_params, &gas_params, context, ty_args, args)
        },
    )
}

#[derive(Debug, Clone)]
pub struct BorrowGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
}

fn native_borrow(
    common_gas_params: &CommonGasParameters,
    gas_params: &BorrowGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 1);
    assert_eq!(args.len(), 1);

    let object_context = context.extensions().get::<NativeObjectContext>();
    let mut object_data = object_context.object_data.borrow_mut();

    let address = pop_arg!(args, AccountAddress);
    let object = object_data.get_or_create_object(address)?;

    let mut cost = gas_params.base;

    let key = ty_args.pop().unwrap();
    let struct_tag = match context.type_to_type_tag(&key)? {
        TypeTag::Struct(struct_tag) => *struct_tag,
        _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
    };
    // let key_bytes = bcs::to_bytes(&context.type_to_type_tag(&key)?).map_err(|_| partial_extension_error("cannot serialize object type tag"))?;
    // cost += gas_params.per_byte_serialized * NumBytes::new(key_bytes.len() as u64);

    let value_layout = get_type_layout(context, &key)?;
    let (gv, loaded) = object.get_or_create_global_value(context, struct_tag, value_layout)?;
    cost += common_gas_params.calculate_load_cost(loaded);

    match gv.borrow_global() {
        Ok(ref_val) => Ok(NativeResult::ok(cost, smallvec![ref_val])),
        Err(_) => Ok(NativeResult::err(cost, NOT_FOUND)),
    }
}

pub fn make_native_borrow(
    common_gas_params: CommonGasParameters,
    gas_params: BorrowGasParameters,
) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_borrow(&common_gas_params, &gas_params, context, ty_args, args)
        },
    )
}

#[derive(Debug, Clone)]
pub struct ExistsGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
}

fn native_exists(
    common_gas_params: &CommonGasParameters,
    gas_params: &ExistsGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 1);
    assert_eq!(args.len(), 1);

    let object_context = context.extensions().get::<NativeObjectContext>();
    let mut object_data = object_context.object_data.borrow_mut();

    let address = pop_arg!(args, AccountAddress);
    let object = object_data.get_or_create_object(address)?;

    let mut cost = gas_params.base;

    let key = ty_args.pop().unwrap();
    let struct_tag = match context.type_to_type_tag(&key)? {
        TypeTag::Struct(struct_tag) => *struct_tag,
        _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
    };
    // let key_bytes = bcs::to_bytes(&context.type_to_type_tag(&key)?).map_err(|_| partial_extension_error("cannot serialize object type tag"))?;
    // cost += gas_params.per_byte_serialized * NumBytes::new(key_bytes.len() as u64);

    let value_layout = get_type_layout(context, &key)?;
    let (gv, loaded) = object.get_or_create_global_value(context, struct_tag, value_layout)?;
    cost += common_gas_params.calculate_load_cost(loaded);

    let exists = Value::bool(gv.exists()?);

    Ok(NativeResult::ok(cost, smallvec![exists]))
}

pub fn make_native_exists(
    common_gas_params: CommonGasParameters,
    gas_params: ExistsGasParameters,
) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_exists(&common_gas_params, &gas_params, context, ty_args, args)
        },
    )
}

#[derive(Debug, Clone)]
pub struct RemoveGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
}

fn native_remove(
    common_gas_params: &CommonGasParameters,
    gas_params: &RemoveGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 1);
    assert_eq!(args.len(), 1);

    let object_context = context.extensions().get::<NativeObjectContext>();
    let mut object_data = object_context.object_data.borrow_mut();

    let address = pop_arg!(args, AccountAddress);
    let object = object_data.get_or_create_object(address)?;

    let mut cost = gas_params.base;

    let key = ty_args.pop().unwrap();
    let struct_tag = match context.type_to_type_tag(&key)? {
        TypeTag::Struct(struct_tag) => *struct_tag,
        _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
    };
    // let key_bytes = bcs::to_bytes(&context.type_to_type_tag(&key)?).map_err(|_| partial_extension_error("cannot serialize object type tag"))?;
    // cost += gas_params.per_byte_serialized * NumBytes::new(key_bytes.len() as u64);

    let value_layout = get_type_layout(context, &key)?;
    let (gv, loaded) = object.get_or_create_global_value(context, struct_tag, value_layout)?;
    cost += common_gas_params.calculate_load_cost(loaded);

    match gv.move_from() {
        Ok(val) => Ok(NativeResult::ok(cost, smallvec![val])),
        Err(_) => Ok(NativeResult::err(cost, NOT_FOUND)),
    }
}

pub fn make_native_remove(
    common_gas_params: CommonGasParameters,
    gas_params: RemoveGasParameters,
) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_remove(&common_gas_params, &gas_params, context, ty_args, args)
        },
    )
}


#[derive(Debug, Clone)]
pub struct GasParameters {
    pub common: CommonGasParameters,
    pub add: AddGasParameters,
    pub borrow: BorrowGasParameters,
    pub exists: ExistsGasParameters,
    pub remove: RemoveGasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            common: CommonGasParameters {
                load_base: 0.into(),
                load_per_byte: 0.into(),
                load_failure: 0.into(),
            },
            add: AddGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
            borrow: BorrowGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
            exists: ExistsGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
            remove: RemoveGasParameters {
                base: 0.into(),
                per_byte_serialized: 0.into(),
            },
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
    (
        "native_add",
        make_native_add(gas_params.common.clone(), gas_params.add),
    ),
    (
        "native_borrow",
        make_native_borrow(gas_params.common.clone(), gas_params.borrow.clone()),
    ),
    (
        "native_borrow_mut",
        make_native_borrow(gas_params.common.clone(), gas_params.borrow),
    ),
    (
        "native_remove",
        make_native_remove(gas_params.common.clone(), gas_params.remove),
    ),
    (
        "native_exists",
        make_native_exists(gas_params.common, gas_params.exists),
    )
    ];
    crate::natives::helpers::make_module_natives(natives)
}
fn serialize(layout: &MoveTypeLayout, val: &Value) -> PartialVMResult<Vec<u8>> {
    val.simple_serialize(layout)
        .ok_or_else(|| partial_extension_error("cannot serialize object value"))
}

fn deserialize(layout: &MoveTypeLayout, bytes: &[u8]) -> PartialVMResult<Value> {
    Value::simple_deserialize(bytes, layout)
        .ok_or_else(|| partial_extension_error("cannot deserialize object value"))
}

fn partial_extension_error(msg: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(msg.to_string())
}

fn get_type_layout(context: &NativeContext, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
    context
        .type_to_type_layout(ty)?
        .ok_or_else(|| partial_extension_error("cannot determine type layout"))
}
