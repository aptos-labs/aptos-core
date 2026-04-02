// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod access_specifier_loader;

mod function;
pub use function::{Function, LoadedFunction, LoadedFunctionOwner};
pub(crate) use function::{
    FunctionHandle, FunctionInstantiation, FunctionPtr, GenericFunctionPtr, LazyLoadedFunction,
    LazyLoadedFunctionState,
};

mod modules;
pub use modules::Module;
pub(crate) use modules::{StructVariantInfo, VariantFieldInfo};

mod script;
pub use script::Script;

mod single_signature_loader;

mod type_loader;
use type_loader::convert_tok_to_type;
