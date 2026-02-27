// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Aptos API response / error handling philosophy.
//!
//! The return type for every endpoint should be a
//! `Result<AptosResponse<T>, AptosErrorResponse>` where the error type
//! implements the relevant error traits so that callers can construct
//! errors with the appropriate HTTP status code at compile time.
//!
//! Every endpoint should be able to return data as JSON or BCS, depending on
//! the Accept header given by the client. If none is given, default to JSON.
//!
//! Where possible, if the API is returning data as BCS, it should pull the
//! bytes directly from the DB where possible without further processing.
//!
//! Internally, there are many functions that can return errors. The context
//! for what type of error those functions return is lost if we return an
//! opaque error type like anyhow::Error. As such, it is important that each
//! function return error types that capture the intended status code. This
//! module defines traits to help with this, ensuring that the error types
//! returned by each function and its callers is enforced at compile time.
//! See generate_error_traits and its invocations for more on this topic.

use aptos_api_types::{Address, AptosErrorCode, LedgerInfo};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::StructTag,
};
use serde_json::Value;
use std::fmt::Display;

/// This trait defines common functions that all error responses should impl.
/// Mostly these are helpers to allow callers holding an error response to
/// manipulate the AptosError inside it.
pub trait AptosErrorResponse {
    fn inner_mut(&mut self) -> &mut aptos_api_types::AptosError;
}

/// This macro defines traits for all of the given status codes. In each trait
/// there is a function that defines a helper for building an instance of the
/// error type using that code. These traits are helpful for defining what
/// error types an internal function can return. For example, the failpoint
/// function can return an internal error, so its signature would look like:
/// fn fail_point<E: InternalError>(name: &str) -> anyhow::Result<(), E>
/// This should be invoked just once, taking in every status that we use
/// throughout the entire API. Every one of these traits requires that the
/// implementor also implements AptosErrorResponse, which saves functions from
/// having to add that bound to errors themselves.
#[macro_export]
macro_rules! generate_error_traits {
    ($($trait_name:ident),*) => {
        paste::paste! {
        $(
        pub trait [<$trait_name Error>]: AptosErrorResponse {
            #[allow(unused)]
            fn [<$trait_name:snake _with_code>]<Err: std::fmt::Display>(
                err: Err,
                error_code: aptos_api_types::AptosErrorCode,
                ledger_info: &aptos_api_types::LedgerInfo,
            ) -> Self where Self: Sized;

            #[allow(unused)]
            fn [<$trait_name:snake _with_code_no_info>]<Err: std::fmt::Display>(
                err: Err,
                error_code: aptos_api_types::AptosErrorCode,
            ) -> Self where Self: Sized;

            #[allow(unused)]
            fn [<$trait_name:snake _with_optional_vm_status_and_ledger_info>]<Err: std::fmt::Display>(
                err: Err,
                error_code: aptos_api_types::AptosErrorCode,
                vm_status: Option<aptos_types::vm_status::StatusCode>,
                ledger_info: Option<&aptos_api_types::LedgerInfo>
            ) -> Self where Self: Sized;

            #[allow(unused)]
            fn [<$trait_name:snake _with_vm_status>]<Err: std::fmt::Display>(
                err: Err,
                error_code: aptos_api_types::AptosErrorCode,
                vm_status: aptos_types::vm_status::StatusCode,
                ledger_info: &aptos_api_types::LedgerInfo
            ) -> Self where Self: Sized;

            #[allow(unused)]
            fn [<$trait_name:snake _from_aptos_error>](
                aptos_error: aptos_api_types::AptosError,
                ledger_info: &aptos_api_types::LedgerInfo
            ) -> Self where Self: Sized;
        }
        )*
        }
    };
}

generate_error_traits!(
    BadRequest,
    Gone,
    NotFound,
    Forbidden,
    PayloadTooLarge,
    Internal,
    InsufficientStorage,
    ServiceUnavailable
);

pub trait StdApiError: NotFoundError + GoneError + InternalError + ServiceUnavailableError {}
impl<T> StdApiError for T where
    T: NotFoundError + GoneError + InternalError + ServiceUnavailableError
{
}

pub fn build_not_found<S: Display, E: NotFoundError>(
    resource: &str,
    identifier: S,
    error_code: AptosErrorCode,
    ledger_info: &LedgerInfo,
) -> E {
    E::not_found_with_code(
        format!("{} not found by {}", resource, identifier),
        error_code,
        ledger_info,
    )
}

pub fn json_api_disabled<S: Display, E: ForbiddenError>(identifier: S) -> E {
    E::forbidden_with_code_no_info(
        format!(
            "{} with JSON output is disabled on this endpoint",
            identifier
        ),
        AptosErrorCode::ApiDisabled,
    )
}

pub fn bcs_api_disabled<S: Display, E: ForbiddenError>(identifier: S) -> E {
    E::forbidden_with_code_no_info(
        format!(
            "{} with BCS output is disabled on this endpoint",
            identifier
        ),
        AptosErrorCode::ApiDisabled,
    )
}

pub fn version_not_found<E: NotFoundError>(ledger_version: u64, ledger_info: &LedgerInfo) -> E {
    build_not_found(
        "Ledger version",
        format!("Ledger version({})", ledger_version),
        AptosErrorCode::VersionNotFound,
        ledger_info,
    )
}

pub fn version_pruned<E: GoneError>(ledger_version: u64, ledger_info: &LedgerInfo) -> E {
    E::gone_with_code(
        format!("Ledger version({}) has been pruned", ledger_version),
        AptosErrorCode::VersionPruned,
        ledger_info,
    )
}

pub fn account_not_found<E: NotFoundError>(
    address: Address,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Account",
        format!(
            "Address({}) and Ledger version({})",
            address, ledger_version
        ),
        AptosErrorCode::AccountNotFound,
        ledger_info,
    )
}

pub fn resource_not_found<E: NotFoundError>(
    address: Address,
    struct_tag: &StructTag,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Resource",
        format!(
            "Address({}), Struct tag({}) and Ledger version({})",
            address,
            struct_tag.to_canonical_string(),
            ledger_version
        ),
        AptosErrorCode::ResourceNotFound,
        ledger_info,
    )
}

pub fn module_not_found<E: NotFoundError>(
    address: Address,
    module_name: &IdentStr,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Module",
        format!(
            "Address({}), Module name({}) and Ledger version({})",
            address, module_name, ledger_version
        ),
        AptosErrorCode::ModuleNotFound,
        ledger_info,
    )
}

pub fn struct_field_not_found<E: NotFoundError>(
    address: Address,
    struct_tag: &StructTag,
    field_name: &Identifier,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Struct Field",
        format!(
            "Address({}), Struct tag({}), Field name({}) and Ledger version({})",
            address,
            struct_tag.to_canonical_string(),
            field_name,
            ledger_version
        ),
        AptosErrorCode::StructFieldNotFound,
        ledger_info,
    )
}

pub fn table_item_not_found<E: NotFoundError>(
    table_handle: Address,
    table_key: &Value,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Table Item",
        format!(
            "Table handle({}), Table key({}) and Ledger version({})",
            table_handle, table_key, ledger_version
        ),
        AptosErrorCode::TableItemNotFound,
        ledger_info,
    )
}

pub fn block_not_found_by_height<E: NotFoundError>(
    block_height: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Block",
        format!("Block height({})", block_height,),
        AptosErrorCode::BlockNotFound,
        ledger_info,
    )
}

pub fn block_not_found_by_version<E: NotFoundError>(
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> E {
    build_not_found(
        "Block",
        format!("Ledger version({})", ledger_version,),
        AptosErrorCode::BlockNotFound,
        ledger_info,
    )
}

pub fn block_pruned_by_height<E: GoneError>(block_height: u64, ledger_info: &LedgerInfo) -> E {
    E::gone_with_code(
        format!("Block({}) has been pruned", block_height),
        AptosErrorCode::BlockPruned,
        ledger_info,
    )
}
