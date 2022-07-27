// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// This macro helps implement necessary traits for a type to be used in
/// a struct that is used in a poem-openapi server. Where possible, prefer
/// to use the existing poem-openapi macros such as Object, NewType, etc.
/// This macro erases the type information, instead returning the data as
/// a string in its JSON representation. For newtypes wrapping strings,
/// this is perfect, but otherwise this is a bit scary, so use it with caution.
#[macro_export]
macro_rules! impl_poem_type {
    ($($ty:ty),*) => {
        $(
        impl ::poem_openapi::types::Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> std::borrow::Cow<'static, str> {
                format!("string({})", stringify!($ty)).into()
            }

            fn schema_ref() -> ::poem_openapi::registry::MetaSchemaRef {
                ::poem_openapi::registry::MetaSchemaRef::Inline(Box::new(::poem_openapi::registry::MetaSchema::new_with_format("string", stringify!($ty))))
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

        )*
    };
}

// This macro implements the traits necessary for using a type as a parameter
// in a poem-openapi endpoint handler, specifically as an argument like Path<T>.
// A type must impl FromStr for this to work, hence why it is a seperate macro.
#[macro_export]
macro_rules! impl_poem_parameter {
    ($($ty:ty),*) => {
        $(
        impl ::poem_openapi::types::ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ::poem_openapi::types::ParseResult<Self> {
                value.parse().map_err(::poem_openapi::types::ParseError::custom)
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
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    #[derive(Deserialize, Serialize)]
    struct This {
        value: String,
    }

    #[derive(Deserialize, Serialize)]
    struct That(String);

    impl FromStr for That {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(That(s.to_string()))
        }
    }

    #[test]
    fn test() {
        impl_poem_type!(This);

        impl_poem_type!(That);
        impl_poem_parameter!(That);
    }
}
