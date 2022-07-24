// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! The Aptos API response / error handling philosophy.
//!
//! The return type for every endpoint should be a
//! poem::Result<MyResponse<T>, MyError> where MyResponse is an instance of
//! ApiResponse that contains only the status codes that it can actually
//! return. This will manifest in the OpenAPI spec, making it clear to users
//! what the API can actually return. The error should operate the same way,
//! where it only describes error codes that it can actually return.
//!
//! Every endpoint should be able to return data as JSON or BCS, depending on
//! the Accept header given by the client. If none is given, default to JSON.
//!
//! The client should be able to provide data to POST endpoints as either JSON
//! or BCS, given they provide the appropriate Content-Type header.
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

// TODO: Pending further discussion with the team, migrate back to U64 over u64.

use std::fmt::Display;

use super::accept_type::AcceptType;
use aptos_api_types::U64;
use poem_openapi::{payload::Json, types::ToJSON, Enum, Object, ResponseContent};

use super::bcs_payload::Bcs;

/// This is the generic struct we use for all API errors, it contains a string
/// message and an Aptos API specific error code.
#[derive(Debug, Object)]
pub struct AptosError {
    message: String,
    error_code: Option<AptosErrorCode>,
    aptos_ledger_version: Option<U64>,
}

impl AptosError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            error_code: None,
            aptos_ledger_version: None,
        }
    }
    pub fn error_code(mut self, error_code: AptosErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }

    pub fn aptos_ledger_version(mut self, ledger_version: u64) -> Self {
        self.aptos_ledger_version = Some(ledger_version.into());
        self
    }
}

impl From<anyhow::Error> for AptosError {
    fn from(error: anyhow::Error) -> Self {
        AptosError::new(format!("{:#}", error))
    }
}

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
// Make sure the integer codes increment one by one.
#[derive(Debug, Enum)]
pub enum AptosErrorCode {
    /// The Accept header contained an unsupported Accept type.
    UnsupportedAcceptType = 0,

    /// The API failed to read from storage for this request, not because of a
    /// bad request, but because of some internal error.
    ReadFromStorageError = 1,

    /// The data we read from the DB was not valid BCS.
    InvalidBcsInStorageError = 2,

    /// We were unexpectedly unable to convert a Rust type to BCS.
    BcsSerializationError = 3,

    /// The start param given for paging is invalid.
    InvalidStartParam = 4,

    /// The limit param given for paging is invalid.
    InvalidLimitParam = 5,
}

#[derive(ResponseContent)]
pub enum AptosResponseContent<T: ToJSON + Send + Sync> {
    // When returning data as JSON, we take in T and then serialize to JSON
    // as part of the response.
    Json(Json<T>),

    // When returning data as BCS, we never actually interact with the Rust
    // type. Instead, we just return the bytes we read from the DB directly,
    // for efficiency reasons. Only through the `schema` decalaration at the
    // endpoints does the return type make its way into the OpenAPI spec.
    #[oai(actual_type = "Bcs<T>")]
    Bcs(Bcs<Vec<u8>>),
}

/// This trait defines common functions that all error responses should impl.
/// As a user you shouldn't worry about this, the generate_error_response macro
/// takes care of it for you. Mostly these are helpers to allow callers holding
/// an error response to manipulate the AptosError inside it.
pub trait AptosErrorResponse {
    fn inner_mut(&mut self) -> &mut AptosError;

    fn error_code(mut self, error_code: AptosErrorCode) -> Self
    where
        Self: Sized,
    {
        self.inner_mut().error_code = Some(error_code);
        self
    }

    fn aptos_ledger_version(mut self, aptos_ledger_version: u64) -> Self
    where
        Self: Sized,
    {
        self.inner_mut().aptos_ledger_version = Some(aptos_ledger_version.into());
        self
    }
}

/// This macro defines traits for all of the given status codes. In eahc trait
/// there is a function that defines a helper for building an instance of the
/// error type using that code. These traits are helpful for defining what
/// error types an internal function can return. For example, the failpoint
/// function can return an internal error, so its signature would look like:
/// fn fail_point_poem<E: InternalError>(name: &str) -> anyhow::Result<(), E>
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
            fn [<$trait_name:snake>](error: anyhow::Error) -> Self where Self: Sized;
            fn [<$trait_name:snake _str>](error_str: &str) -> Self where Self: Sized;
        }
        )*
        }
    };
}

/// This macro helps generate types, From impls, etc. for an error
/// response from a Poem endpoint. It generates a response type that only has
/// the specified response codes, which is then reflected in the OpenAPI spec.
/// For each status code given to a particular invocation of this macro, we
/// implement the relevant trait from generate_error_traits.
/// See the comments in the macro for an explanation of what is happening.
#[macro_export]
macro_rules! generate_error_response {
    ($enum_name:ident, $(($status:literal, $name:ident)),*) => {
        // We use the paste crate to allows us to generate the name of the code
        // enum, more on that in the comment above that enum.
        paste::paste! {

        // Generate an enum with name `enum_name`. Iterate through each of the
        // response codes, generating a variant for each with the given name
        // and status code. We always generate a variant for 500.
        #[derive(Debug, poem_openapi::ApiResponse)]
        pub enum $enum_name {
            $(
            #[oai(status = $status)]
            $name(poem_openapi::payload::Json<$crate::poem_backend::AptosError>),
            )*
        }

        // For each status, implement the relevant error trait. This means if
        // the macro invocation specifies Internal and BadRequest, the
        // functions internal(anyhow::Error) and bad_request(anyhow::Error)
        // will be generated. There are also variants for taking in strs.
        $(
        impl $crate::poem_backend::[<$name Error>] for $enum_name {
            fn [<$name:snake>](error: anyhow::Error) -> Self where Self: Sized {
                let error = $crate::poem_backend::AptosError::from(error);
                let payload = poem_openapi::payload::Json(error);
                Self::from($enum_name::$name(payload))
            }

            fn [<$name:snake _str>](error_str: &str) -> Self where Self: Sized {
                let error = $crate::poem_backend::AptosError::new(error_str.to_string());
                let payload = poem_openapi::payload::Json(error);
                Self::from($enum_name::$name(payload))
            }
        }
        )*
        }

        // Generate a function that helps get the AptosError within.
        impl $crate::poem_backend::AptosErrorResponse for $enum_name {
            fn inner_mut(&mut self) -> &mut $crate::poem_backend::AptosError {
                match self {
                    $(
                    $enum_name::$name(poem_openapi::payload::Json(inner)) => inner,
                    )*
                }
            }
        }
    };
}

/// This macro helps generate types, From impls, etc. for a successful response
/// from a Poem endpoint. It generates a response type that only has the
/// specified response codes, which is then reflected in the OpenAPI spec.
/// See the comments in the macro for an explanation of what is happening.
#[macro_export]
macro_rules! generate_success_response {
    ($enum_name:ident, $(($status:literal, $name:ident)),*) => {
        // We use the paste crate to allows us to generate the name of the code
        // enum, more on that in the comment above that enum.
        paste::paste! {

        // Generate an enum with name `enum_name`. Iterate through each of the
        // response codes, generating a variant for each with the given name
        // and status code. This extra derive, bad_request_handler, tells the
        // framework to invoke the given function when the framework tries to
        // return some kind of bad request error, e.g. from parsing params.
        // This allows us to convert the framework error into our custom
        // response type. Without this the framework would return errors that
        // don't conform to our OpenAPI spec.
        #[derive(poem_openapi::ApiResponse)]
        #[oai(bad_request_handler = "Self::bad_request_handler")]
        pub enum $enum_name<T: poem_openapi::types::ToJSON + Send + Sync> {
            $(
            #[oai(status = $status)]
            $name(
                $crate::poem_backend::AptosResponseContent<T>,
                #[oai(header = "X-Aptos-Chain-Id")] u16,
                #[oai(header = "X-Aptos-Ledger-Version")] U64,
                #[oai(header = "X-Aptos-Ledger-Oldest-Version")] U64,
                #[oai(header = "X-Aptos-Ledger-TimestampUsec")] U64,
                #[oai(header = "X-Aptos-Epoch")] U64,
            ),
            )*

            // For any endpoint, it is possible for the framework to return a
            // an error repesenting a bad request. As such, to enable us to use
            // bad_request_handler, we include this status code here, since the
            // framework expects this on the T response, not the E. All other
            // errors should be included in the error response, not this one.
            #[oai(status = 400)]
            BadRequest(Json<crate::poem_backend::AptosError>),
        }

        impl<T: poem_openapi::types::ToJSON + Send + Sync> $enum_name<T> {
            // Generate a function that converts the framework-generated error
            // into our custom error response (JSON + AptosError).
            pub fn bad_request_handler(error: poem::Error) -> $enum_name<T> {
                $enum_name::BadRequest(poem_openapi::payload::Json(
                    $crate::poem_backend::AptosError::new(error.to_string()),
                ))
            }
        }

        // Generate an enum that captures all the different status codes that
        // this response type supports. To explain this funky syntax, if you
        // named the main enum MyResponse, this would become MyResponseCode.
        pub enum [<$enum_name Status>] {
            $(
            $name,
            )*
        }

        // Generate a From impl that builds a response from AptosResponseContent.
        // Each variant in the main enum takes in the same argument, so the macro
        // is really just helping us enumerate and build each variant. We use this
        // in the other From impls.
        impl <T: poem_openapi::types::ToJSON + Send + Sync> From<($crate::poem_backend::AptosResponseContent<T>, &aptos_api_types::LedgerInfo, [<$enum_name Status>])>
            for $enum_name<T>
        {
            fn from(
                (value, ledger_info, status): (
                    $crate::poem_backend::AptosResponseContent<T>,
                    &aptos_api_types::LedgerInfo,
                    [<$enum_name Status>]
                ),
            ) -> Self {
                match status {
                    $(
                    [<$enum_name Status>]::$name => {
                        $enum_name::$name(
                            value,
                            ledger_info.chain_id as u16,
                            ledger_info.ledger_version,
                            ledger_info.oldest_ledger_version,
                            ledger_info.ledger_timestamp,
                            ledger_info.epoch,
                        )
                    },
                    )*
                }
            }
        }

        // Generate a From impl that builds a response from a Json<T> and friends.
        impl<T: poem_openapi::types::ToJSON + Send + Sync> From<(poem_openapi::payload::Json<T>, &aptos_api_types::LedgerInfo, [<$enum_name Status>])>
            for $enum_name<T>
        {
            fn from(
                (value, ledger_info, status): (poem_openapi::payload::Json<T>, &aptos_api_types::LedgerInfo, [<$enum_name Status>]),
            ) -> Self {
                let content = $crate::poem_backend::AptosResponseContent::Json(value);
                Self::from((content, ledger_info, status))
            }
        }

        // Generate a From impl that builds a response from a Bcs<Vec<u8>> and friends.
        impl<T: poem_openapi::types::ToJSON + Send + Sync> From<($crate::poem_backend::bcs_payload::Bcs<Vec<u8>>, &aptos_api_types::LedgerInfo, [<$enum_name Status>])>
            for $enum_name<T>
        {
            fn from(
                (value, ledger_info, status): (
                    $crate::poem_backend::bcs_payload::Bcs<Vec<u8>>,
                    &aptos_api_types::LedgerInfo,
                    [<$enum_name Status>]
                ),
            ) -> Self {
                let content = $crate::poem_backend::AptosResponseContent::Bcs(value);
                Self::from((content, ledger_info, status))
            }
        }

        // Generate a TryFrom impl that builds a response from a T, an AcceptType,
        // and all the other usual suspects. It expects to be called with a generic
        // parameter E: InternalError, with which we can build an internal error
        // response in case the BCS serialization fails.
        impl<T: poem_openapi::types::ToJSON + Send + Sync + serde::Serialize> $enum_name<T> {
            pub fn try_from_rust_value<E: InternalError>(
                (value, ledger_info, status, accept_type): (
                    T,
                    &aptos_api_types::LedgerInfo,
                    [<$enum_name Status>],
                    &$crate::poem_backend::AcceptType
                ),
            ) -> Result<Self, E> {
                match accept_type {
                    AcceptType::Bcs => Ok(Self::from((
                        $crate::poem_backend::bcs_payload::Bcs(
                            bcs::to_bytes(&value)
                                .map_err(|e| E::internal(e.into()).error_code($crate::poem_backend::AptosErrorCode::BcsSerializationError))?
                        ),
                        ledger_info,
                        status
                    ))),
                    AcceptType::Json => Ok(Self::from((
                        poem_openapi::payload::Json(value),
                        ledger_info,
                        status
                    ))),
                }
            }
        }
        }
    };
}

// Generate a success response that only has an option for 200.
generate_success_response!(BasicResponse, (200, Ok));

// Generate traits defining a "from" function for each of these status types.
// The error response then impls these traits for each status type they mention.
generate_error_traits!(
    BadRequest,
    NotFound,
    PayloadTooLarge,
    UnsupportedMediaType,
    Internal,
    InsufficientStorage
);

// Generate an error response that only has options for 400 and 500.
generate_error_response!(BasicError, (400, BadRequest), (500, Internal));

// This type just simplifies using BasicResponse and BasicError together.
pub type BasicResult<T> = poem::Result<BasicResponse<T>, BasicError>;

// As above but with 404.
generate_error_response!(
    BasicErrorWith404,
    (400, BadRequest),
    (404, NotFound),
    (500, Internal)
);
pub type BasicResultWith404<T> = poem::Result<BasicResponse<T>, BasicErrorWith404>;

// Just this one helper for a specific kind of 404.
pub fn build_not_found<S: Display, E: NotFoundError>(
    resource: &str,
    identifier: S,
    ledger_version: u64,
) -> E {
    E::not_found_str(&format!("{} not found by {}", resource, identifier))
        .aptos_ledger_version(ledger_version)
}
