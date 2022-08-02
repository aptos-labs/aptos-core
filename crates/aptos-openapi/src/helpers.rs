// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! In order to use a type with poem-openapi, it must implement a certain set
//! of traits, depending on the context in which you want to use the type.
//! For example, if you want to use a type in a struct that you return from
//! an endpoint, it must implement Type. Normally you get this by deriving
//! traits such as Object, Enum, Union, etc. However, in some cases, it is
//! not feasible to use these derives.
//!
//!   - The type is outside of reach, e.g. in a crate in aptos-core that is
//!     too unrelated, or even worse, in a totally different crate (the move
//!     types are a great example of this).
//!   - The type is not expressible via OpenAPI. For example, an enum that
//!     has some enum variants with values and others without values.This is
//!     not allowed in OpenAPI, types must be either unions (variants with
//!     values) or enums (variants without values).
//!   - We would prefer to serialize the data differently than its standard
//!     representation. HexEncodedBytes is a good example of this. Internally,
//!     this is a Vec<u8>, but we know it is hex and prefer to represent it as
//!     a 0x string.
//!
//! For those cases, we have these macros. We can use these to implement the
//! necessary traits for using these types with poem-openapi, without using
//! the derives.
//!
//! Each macro explains itself in further detail.

/// This macro allows you to use a type in a request / response type for use
/// with poem-openapi. In order to use this macro, your type must implement
/// Serialize and Deserialize, so we can encode it as JSON / a string.
///
/// With this macro, you can express what OpenAPI type you want your type to be
/// expressed as in the spec. For example, if your type serializes just to a
/// string, you likely want to invoke the macro like this:
///
///   impl_poem_type!(MyType, "string", ());
///
/// If your type is more complex, and you'd rather it become an "object" in the
/// spec, you should invoke the macro like this:
///
///   impl_poem_type!(MyType, "object", ());
///
/// This macro supports applying additional information to the generated type.
/// For example, you could invoke the macro like this:
///
///   impl_poem_type!(
///       HexEncodedBytes,
///       "string",
///       (
///           example = Some(serde_json::Value::String(
///               "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1".to_string())),
///           description = Some("A hex encoded string"),
///       )
///   );
///
/// To see what different metadata you can apply to the generated type in the
/// spec, take a look at MetaSchema here:
/// https://github.com/poem-web/poem/blob/master/poem-openapi/src/registry/mod.rs
#[macro_export]
macro_rules! impl_poem_type {
    ($ty:ty, $spec_type:literal, ($($key:ident = $value:expr),*)) => {

        impl ::poem_openapi::types::Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> std::borrow::Cow<'static, str> {
                format!("string({})", stringify!($ty)).into()
            }

            // We generate a MetaSchema for our type so we can use it as a
            // a reference in `schema_ref`. The alternative is `schema_ref`
            // generates its own schema there and uses it inline, which leads
            // to lots of repetition in the spec.
            //
            // For example:
            //
            //   gas_unit_price:
            //     $ref: "#/components/schemas/U64"
            //
            // Which refers to:
            //
            //   components:
            //       U64:
            //         type: string
            //         pattern: [0-9]+
            fn register(registry: &mut poem_openapi::registry::Registry) {
                registry.create_schema::<Self, _>(stringify!($ty).to_string(), |_registry| {
                    #[allow(unused_mut)]
                    let mut meta_schema = poem_openapi::registry::MetaSchema::new($spec_type);
                    $(
                    meta_schema.$key = $value;
                    )*
                    meta_schema
                })
            }

            // This function determines what the schema looks like when this
            // type appears in the spec. In our case, it will look like a
            // a reference to the type we generate in the spec.
            fn schema_ref() -> ::poem_openapi::registry::MetaSchemaRef {
                ::poem_openapi::registry::MetaSchemaRef::Reference(format!("{}", stringify!($ty)))
            }

            fn as_raw_value(&self) -> Option<&Self::RawValueType> {
                Some(self)
            }

            fn raw_element_iter<'a>(
                &'a self,
            ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                Box::new(self.as_raw_value().into_iter())
            }
        }

        impl ::poem_openapi::types::ParseFromJSON for $ty {
            fn parse_from_json(value: Option<serde_json::Value>) -> ::poem_openapi::types::ParseResult<Self> {
                let value = value.unwrap_or_default();
                Ok(::serde_json::from_value(value)?)
            }
        }

        impl ::poem_openapi::types::ToJSON for $ty {
            fn to_json(&self) -> Option<serde_json::Value> {
                serde_json::to_value(self).ok()
            }
        }

        impl ::poem_openapi::types::ToHeader for $ty {
            fn to_header(&self) -> Option<::poem::http::HeaderValue> {
                let string = serde_json::to_value(self).ok()?.to_string();
                ::poem::http::HeaderValue::from_str(&string).ok()
            }
        }
    };
}

/// This macro implements the traits necessary for using a type as a parameter
/// in a poem-openapi endpoint handler, specifically as an argument like Path<T>.
/// A type must impl FromStr for this to work, hence why it is a seperate macro.
#[macro_export]
macro_rules! impl_poem_parameter {
    ($($ty:ty),*) => {
        $(
        impl ::poem_openapi::types::ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ::poem_openapi::types::ParseResult<Self> {
                $crate::percent_encoding::percent_decode_str(value)
                    .decode_utf8()
                    .map_err(::poem_openapi::types::ParseError::custom)?
                    .parse()
                    .map_err(::poem_openapi::types::ParseError::custom)
            }
        }

        #[async_trait::async_trait]
        impl ::poem_openapi::types::ParseFromMultipartField for $ty {
            async fn parse_from_multipart(field: Option<::poem::web::Field>) -> ::poem_openapi::types::ParseResult<Self> {
                match field {
                    Some(field) => Ok(field.text().await?.parse()?),
                    None => Err(::poem_openapi::types::ParseError::expected_input()),
                }
            }
        }

        )*
    };
}

mod test {
    #[allow(unused_imports)]
    use poem_openapi::types::ParseFromParameter;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    #[derive(Debug, Deserialize, Serialize)]
    struct This {
        value: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct That(pub String);

    impl FromStr for That {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(That(s.to_string()))
        }
    }

    #[test]
    fn test() {
        impl_poem_type!(This, "string", ());

        impl_poem_type!(That, "string", ());
        impl_poem_parameter!(That);

        assert_eq!(
            That::parse_from_parameter("0x1::coin::CoinStore::%3C0x1::aptos_coin::AptosCoin%3E")
                .unwrap()
                .0,
            "0x1::coin::CoinStore::<0x1::aptos_coin::AptosCoin>".to_string(),
        );
    }
}
