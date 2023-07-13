// Copyright © Aptos Foundation

// @generated
impl serde::Serialize for ClientLibraryDestination {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "CLIENT_LIBRARY_DESTINATION_UNSPECIFIED",
            Self::Github => "GITHUB",
            Self::PackageManager => "PACKAGE_MANAGER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for ClientLibraryDestination {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "CLIENT_LIBRARY_DESTINATION_UNSPECIFIED",
            "GITHUB",
            "PACKAGE_MANAGER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ClientLibraryDestination;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(ClientLibraryDestination::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(ClientLibraryDestination::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "CLIENT_LIBRARY_DESTINATION_UNSPECIFIED" => Ok(ClientLibraryDestination::Unspecified),
                    "GITHUB" => Ok(ClientLibraryDestination::Github),
                    "PACKAGE_MANAGER" => Ok(ClientLibraryDestination::PackageManager),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ClientLibraryOrganization {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "CLIENT_LIBRARY_ORGANIZATION_UNSPECIFIED",
            Self::Cloud => "CLOUD",
            Self::Ads => "ADS",
            Self::Photos => "PHOTOS",
            Self::StreetView => "STREET_VIEW",
            Self::Shopping => "SHOPPING",
            Self::Geo => "GEO",
            Self::GenerativeAi => "GENERATIVE_AI",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for ClientLibraryOrganization {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "CLIENT_LIBRARY_ORGANIZATION_UNSPECIFIED",
            "CLOUD",
            "ADS",
            "PHOTOS",
            "STREET_VIEW",
            "SHOPPING",
            "GEO",
            "GENERATIVE_AI",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ClientLibraryOrganization;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(ClientLibraryOrganization::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(ClientLibraryOrganization::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "CLIENT_LIBRARY_ORGANIZATION_UNSPECIFIED" => Ok(ClientLibraryOrganization::Unspecified),
                    "CLOUD" => Ok(ClientLibraryOrganization::Cloud),
                    "ADS" => Ok(ClientLibraryOrganization::Ads),
                    "PHOTOS" => Ok(ClientLibraryOrganization::Photos),
                    "STREET_VIEW" => Ok(ClientLibraryOrganization::StreetView),
                    "SHOPPING" => Ok(ClientLibraryOrganization::Shopping),
                    "GEO" => Ok(ClientLibraryOrganization::Geo),
                    "GENERATIVE_AI" => Ok(ClientLibraryOrganization::GenerativeAi),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ClientLibrarySettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.version.is_empty() {
            len += 1;
        }
        if self.launch_stage != 0 {
            len += 1;
        }
        if self.rest_numeric_enums {
            len += 1;
        }
        if self.java_settings.is_some() {
            len += 1;
        }
        if self.cpp_settings.is_some() {
            len += 1;
        }
        if self.php_settings.is_some() {
            len += 1;
        }
        if self.python_settings.is_some() {
            len += 1;
        }
        if self.node_settings.is_some() {
            len += 1;
        }
        if self.dotnet_settings.is_some() {
            len += 1;
        }
        if self.ruby_settings.is_some() {
            len += 1;
        }
        if self.go_settings.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.ClientLibrarySettings", len)?;
        if !self.version.is_empty() {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if self.launch_stage != 0 {
            let v = LaunchStage::from_i32(self.launch_stage)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.launch_stage)))?;
            struct_ser.serialize_field("launchStage", &v)?;
        }
        if self.rest_numeric_enums {
            struct_ser.serialize_field("restNumericEnums", &self.rest_numeric_enums)?;
        }
        if let Some(v) = self.java_settings.as_ref() {
            struct_ser.serialize_field("javaSettings", v)?;
        }
        if let Some(v) = self.cpp_settings.as_ref() {
            struct_ser.serialize_field("cppSettings", v)?;
        }
        if let Some(v) = self.php_settings.as_ref() {
            struct_ser.serialize_field("phpSettings", v)?;
        }
        if let Some(v) = self.python_settings.as_ref() {
            struct_ser.serialize_field("pythonSettings", v)?;
        }
        if let Some(v) = self.node_settings.as_ref() {
            struct_ser.serialize_field("nodeSettings", v)?;
        }
        if let Some(v) = self.dotnet_settings.as_ref() {
            struct_ser.serialize_field("dotnetSettings", v)?;
        }
        if let Some(v) = self.ruby_settings.as_ref() {
            struct_ser.serialize_field("rubySettings", v)?;
        }
        if let Some(v) = self.go_settings.as_ref() {
            struct_ser.serialize_field("goSettings", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ClientLibrarySettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "launch_stage",
            "launchStage",
            "rest_numeric_enums",
            "restNumericEnums",
            "java_settings",
            "javaSettings",
            "cpp_settings",
            "cppSettings",
            "php_settings",
            "phpSettings",
            "python_settings",
            "pythonSettings",
            "node_settings",
            "nodeSettings",
            "dotnet_settings",
            "dotnetSettings",
            "ruby_settings",
            "rubySettings",
            "go_settings",
            "goSettings",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            LaunchStage,
            RestNumericEnums,
            JavaSettings,
            CppSettings,
            PhpSettings,
            PythonSettings,
            NodeSettings,
            DotnetSettings,
            RubySettings,
            GoSettings,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "version" => Ok(GeneratedField::Version),
                            "launchStage" | "launch_stage" => Ok(GeneratedField::LaunchStage),
                            "restNumericEnums" | "rest_numeric_enums" => Ok(GeneratedField::RestNumericEnums),
                            "javaSettings" | "java_settings" => Ok(GeneratedField::JavaSettings),
                            "cppSettings" | "cpp_settings" => Ok(GeneratedField::CppSettings),
                            "phpSettings" | "php_settings" => Ok(GeneratedField::PhpSettings),
                            "pythonSettings" | "python_settings" => Ok(GeneratedField::PythonSettings),
                            "nodeSettings" | "node_settings" => Ok(GeneratedField::NodeSettings),
                            "dotnetSettings" | "dotnet_settings" => Ok(GeneratedField::DotnetSettings),
                            "rubySettings" | "ruby_settings" => Ok(GeneratedField::RubySettings),
                            "goSettings" | "go_settings" => Ok(GeneratedField::GoSettings),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ClientLibrarySettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.ClientLibrarySettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ClientLibrarySettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut launch_stage__ = None;
                let mut rest_numeric_enums__ = None;
                let mut java_settings__ = None;
                let mut cpp_settings__ = None;
                let mut php_settings__ = None;
                let mut python_settings__ = None;
                let mut node_settings__ = None;
                let mut dotnet_settings__ = None;
                let mut ruby_settings__ = None;
                let mut go_settings__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(map.next_value()?);
                        }
                        GeneratedField::LaunchStage => {
                            if launch_stage__.is_some() {
                                return Err(serde::de::Error::duplicate_field("launchStage"));
                            }
                            launch_stage__ = Some(map.next_value::<LaunchStage>()? as i32);
                        }
                        GeneratedField::RestNumericEnums => {
                            if rest_numeric_enums__.is_some() {
                                return Err(serde::de::Error::duplicate_field("restNumericEnums"));
                            }
                            rest_numeric_enums__ = Some(map.next_value()?);
                        }
                        GeneratedField::JavaSettings => {
                            if java_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("javaSettings"));
                            }
                            java_settings__ = map.next_value()?;
                        }
                        GeneratedField::CppSettings => {
                            if cpp_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cppSettings"));
                            }
                            cpp_settings__ = map.next_value()?;
                        }
                        GeneratedField::PhpSettings => {
                            if php_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("phpSettings"));
                            }
                            php_settings__ = map.next_value()?;
                        }
                        GeneratedField::PythonSettings => {
                            if python_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pythonSettings"));
                            }
                            python_settings__ = map.next_value()?;
                        }
                        GeneratedField::NodeSettings => {
                            if node_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nodeSettings"));
                            }
                            node_settings__ = map.next_value()?;
                        }
                        GeneratedField::DotnetSettings => {
                            if dotnet_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dotnetSettings"));
                            }
                            dotnet_settings__ = map.next_value()?;
                        }
                        GeneratedField::RubySettings => {
                            if ruby_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rubySettings"));
                            }
                            ruby_settings__ = map.next_value()?;
                        }
                        GeneratedField::GoSettings => {
                            if go_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("goSettings"));
                            }
                            go_settings__ = map.next_value()?;
                        }
                    }
                }
                Ok(ClientLibrarySettings {
                    version: version__.unwrap_or_default(),
                    launch_stage: launch_stage__.unwrap_or_default(),
                    rest_numeric_enums: rest_numeric_enums__.unwrap_or_default(),
                    java_settings: java_settings__,
                    cpp_settings: cpp_settings__,
                    php_settings: php_settings__,
                    python_settings: python_settings__,
                    node_settings: node_settings__,
                    dotnet_settings: dotnet_settings__,
                    ruby_settings: ruby_settings__,
                    go_settings: go_settings__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.ClientLibrarySettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CommonLanguageSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.reference_docs_uri.is_empty() {
            len += 1;
        }
        if !self.destinations.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.CommonLanguageSettings", len)?;
        if !self.reference_docs_uri.is_empty() {
            struct_ser.serialize_field("referenceDocsUri", &self.reference_docs_uri)?;
        }
        if !self.destinations.is_empty() {
            let v = self.destinations.iter().cloned().map(|v| {
                ClientLibraryDestination::from_i32(v)
                    .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("destinations", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CommonLanguageSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "reference_docs_uri",
            "referenceDocsUri",
            "destinations",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReferenceDocsUri,
            Destinations,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "referenceDocsUri" | "reference_docs_uri" => Ok(GeneratedField::ReferenceDocsUri),
                            "destinations" => Ok(GeneratedField::Destinations),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CommonLanguageSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.CommonLanguageSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CommonLanguageSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut reference_docs_uri__ = None;
                let mut destinations__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ReferenceDocsUri => {
                            if reference_docs_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("referenceDocsUri"));
                            }
                            reference_docs_uri__ = Some(map.next_value()?);
                        }
                        GeneratedField::Destinations => {
                            if destinations__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinations"));
                            }
                            destinations__ = Some(map.next_value::<Vec<ClientLibraryDestination>>()?.into_iter().map(|x| x as i32).collect());
                        }
                    }
                }
                Ok(CommonLanguageSettings {
                    reference_docs_uri: reference_docs_uri__.unwrap_or_default(),
                    destinations: destinations__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.CommonLanguageSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CppSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.CppSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CppSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CppSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.CppSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CppSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(CppSettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.CppSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CustomHttpPattern {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.kind.is_empty() {
            len += 1;
        }
        if !self.path.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.CustomHttpPattern", len)?;
        if !self.kind.is_empty() {
            struct_ser.serialize_field("kind", &self.kind)?;
        }
        if !self.path.is_empty() {
            struct_ser.serialize_field("path", &self.path)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CustomHttpPattern {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "kind",
            "path",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            Path,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "kind" => Ok(GeneratedField::Kind),
                            "path" => Ok(GeneratedField::Path),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CustomHttpPattern;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.CustomHttpPattern")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CustomHttpPattern, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut path__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map.next_value()?);
                        }
                        GeneratedField::Path => {
                            if path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("path"));
                            }
                            path__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(CustomHttpPattern {
                    kind: kind__.unwrap_or_default(),
                    path: path__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.CustomHttpPattern", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DotnetSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        if !self.renamed_services.is_empty() {
            len += 1;
        }
        if !self.renamed_resources.is_empty() {
            len += 1;
        }
        if !self.ignored_resources.is_empty() {
            len += 1;
        }
        if !self.forced_namespace_aliases.is_empty() {
            len += 1;
        }
        if !self.handwritten_signatures.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.DotnetSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        if !self.renamed_services.is_empty() {
            struct_ser.serialize_field("renamedServices", &self.renamed_services)?;
        }
        if !self.renamed_resources.is_empty() {
            struct_ser.serialize_field("renamedResources", &self.renamed_resources)?;
        }
        if !self.ignored_resources.is_empty() {
            struct_ser.serialize_field("ignoredResources", &self.ignored_resources)?;
        }
        if !self.forced_namespace_aliases.is_empty() {
            struct_ser.serialize_field("forcedNamespaceAliases", &self.forced_namespace_aliases)?;
        }
        if !self.handwritten_signatures.is_empty() {
            struct_ser.serialize_field("handwrittenSignatures", &self.handwritten_signatures)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DotnetSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
            "renamed_services",
            "renamedServices",
            "renamed_resources",
            "renamedResources",
            "ignored_resources",
            "ignoredResources",
            "forced_namespace_aliases",
            "forcedNamespaceAliases",
            "handwritten_signatures",
            "handwrittenSignatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
            RenamedServices,
            RenamedResources,
            IgnoredResources,
            ForcedNamespaceAliases,
            HandwrittenSignatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            "renamedServices" | "renamed_services" => Ok(GeneratedField::RenamedServices),
                            "renamedResources" | "renamed_resources" => Ok(GeneratedField::RenamedResources),
                            "ignoredResources" | "ignored_resources" => Ok(GeneratedField::IgnoredResources),
                            "forcedNamespaceAliases" | "forced_namespace_aliases" => Ok(GeneratedField::ForcedNamespaceAliases),
                            "handwrittenSignatures" | "handwritten_signatures" => Ok(GeneratedField::HandwrittenSignatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DotnetSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.DotnetSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DotnetSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                let mut renamed_services__ = None;
                let mut renamed_resources__ = None;
                let mut ignored_resources__ = None;
                let mut forced_namespace_aliases__ = None;
                let mut handwritten_signatures__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                        GeneratedField::RenamedServices => {
                            if renamed_services__.is_some() {
                                return Err(serde::de::Error::duplicate_field("renamedServices"));
                            }
                            renamed_services__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::RenamedResources => {
                            if renamed_resources__.is_some() {
                                return Err(serde::de::Error::duplicate_field("renamedResources"));
                            }
                            renamed_resources__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::IgnoredResources => {
                            if ignored_resources__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ignoredResources"));
                            }
                            ignored_resources__ = Some(map.next_value()?);
                        }
                        GeneratedField::ForcedNamespaceAliases => {
                            if forced_namespace_aliases__.is_some() {
                                return Err(serde::de::Error::duplicate_field("forcedNamespaceAliases"));
                            }
                            forced_namespace_aliases__ = Some(map.next_value()?);
                        }
                        GeneratedField::HandwrittenSignatures => {
                            if handwritten_signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("handwrittenSignatures"));
                            }
                            handwritten_signatures__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DotnetSettings {
                    common: common__,
                    renamed_services: renamed_services__.unwrap_or_default(),
                    renamed_resources: renamed_resources__.unwrap_or_default(),
                    ignored_resources: ignored_resources__.unwrap_or_default(),
                    forced_namespace_aliases: forced_namespace_aliases__.unwrap_or_default(),
                    handwritten_signatures: handwritten_signatures__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.DotnetSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FieldBehavior {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "FIELD_BEHAVIOR_UNSPECIFIED",
            Self::Optional => "OPTIONAL",
            Self::Required => "REQUIRED",
            Self::OutputOnly => "OUTPUT_ONLY",
            Self::InputOnly => "INPUT_ONLY",
            Self::Immutable => "IMMUTABLE",
            Self::UnorderedList => "UNORDERED_LIST",
            Self::NonEmptyDefault => "NON_EMPTY_DEFAULT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for FieldBehavior {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "FIELD_BEHAVIOR_UNSPECIFIED",
            "OPTIONAL",
            "REQUIRED",
            "OUTPUT_ONLY",
            "INPUT_ONLY",
            "IMMUTABLE",
            "UNORDERED_LIST",
            "NON_EMPTY_DEFAULT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FieldBehavior;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(FieldBehavior::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(FieldBehavior::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "FIELD_BEHAVIOR_UNSPECIFIED" => Ok(FieldBehavior::Unspecified),
                    "OPTIONAL" => Ok(FieldBehavior::Optional),
                    "REQUIRED" => Ok(FieldBehavior::Required),
                    "OUTPUT_ONLY" => Ok(FieldBehavior::OutputOnly),
                    "INPUT_ONLY" => Ok(FieldBehavior::InputOnly),
                    "IMMUTABLE" => Ok(FieldBehavior::Immutable),
                    "UNORDERED_LIST" => Ok(FieldBehavior::UnorderedList),
                    "NON_EMPTY_DEFAULT" => Ok(FieldBehavior::NonEmptyDefault),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for GoSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.GoSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GoSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GoSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.GoSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GoSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(GoSettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.GoSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Http {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rules.is_empty() {
            len += 1;
        }
        if self.fully_decode_reserved_expansion {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.Http", len)?;
        if !self.rules.is_empty() {
            struct_ser.serialize_field("rules", &self.rules)?;
        }
        if self.fully_decode_reserved_expansion {
            struct_ser.serialize_field("fullyDecodeReservedExpansion", &self.fully_decode_reserved_expansion)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Http {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rules",
            "fully_decode_reserved_expansion",
            "fullyDecodeReservedExpansion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Rules,
            FullyDecodeReservedExpansion,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "rules" => Ok(GeneratedField::Rules),
                            "fullyDecodeReservedExpansion" | "fully_decode_reserved_expansion" => Ok(GeneratedField::FullyDecodeReservedExpansion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Http;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.Http")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Http, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rules__ = None;
                let mut fully_decode_reserved_expansion__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Rules => {
                            if rules__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rules"));
                            }
                            rules__ = Some(map.next_value()?);
                        }
                        GeneratedField::FullyDecodeReservedExpansion => {
                            if fully_decode_reserved_expansion__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fullyDecodeReservedExpansion"));
                            }
                            fully_decode_reserved_expansion__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(Http {
                    rules: rules__.unwrap_or_default(),
                    fully_decode_reserved_expansion: fully_decode_reserved_expansion__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.Http", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HttpRule {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.selector.is_empty() {
            len += 1;
        }
        if !self.body.is_empty() {
            len += 1;
        }
        if !self.response_body.is_empty() {
            len += 1;
        }
        if !self.additional_bindings.is_empty() {
            len += 1;
        }
        if self.pattern.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.HttpRule", len)?;
        if !self.selector.is_empty() {
            struct_ser.serialize_field("selector", &self.selector)?;
        }
        if !self.body.is_empty() {
            struct_ser.serialize_field("body", &self.body)?;
        }
        if !self.response_body.is_empty() {
            struct_ser.serialize_field("responseBody", &self.response_body)?;
        }
        if !self.additional_bindings.is_empty() {
            struct_ser.serialize_field("additionalBindings", &self.additional_bindings)?;
        }
        if let Some(v) = self.pattern.as_ref() {
            match v {
                http_rule::Pattern::Get(v) => {
                    struct_ser.serialize_field("get", v)?;
                }
                http_rule::Pattern::Put(v) => {
                    struct_ser.serialize_field("put", v)?;
                }
                http_rule::Pattern::Post(v) => {
                    struct_ser.serialize_field("post", v)?;
                }
                http_rule::Pattern::Delete(v) => {
                    struct_ser.serialize_field("delete", v)?;
                }
                http_rule::Pattern::Patch(v) => {
                    struct_ser.serialize_field("patch", v)?;
                }
                http_rule::Pattern::Custom(v) => {
                    struct_ser.serialize_field("custom", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HttpRule {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "selector",
            "body",
            "response_body",
            "responseBody",
            "additional_bindings",
            "additionalBindings",
            "get",
            "put",
            "post",
            "delete",
            "patch",
            "custom",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Selector,
            Body,
            ResponseBody,
            AdditionalBindings,
            Get,
            Put,
            Post,
            Delete,
            Patch,
            Custom,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "selector" => Ok(GeneratedField::Selector),
                            "body" => Ok(GeneratedField::Body),
                            "responseBody" | "response_body" => Ok(GeneratedField::ResponseBody),
                            "additionalBindings" | "additional_bindings" => Ok(GeneratedField::AdditionalBindings),
                            "get" => Ok(GeneratedField::Get),
                            "put" => Ok(GeneratedField::Put),
                            "post" => Ok(GeneratedField::Post),
                            "delete" => Ok(GeneratedField::Delete),
                            "patch" => Ok(GeneratedField::Patch),
                            "custom" => Ok(GeneratedField::Custom),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HttpRule;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.HttpRule")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<HttpRule, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut selector__ = None;
                let mut body__ = None;
                let mut response_body__ = None;
                let mut additional_bindings__ = None;
                let mut pattern__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Selector => {
                            if selector__.is_some() {
                                return Err(serde::de::Error::duplicate_field("selector"));
                            }
                            selector__ = Some(map.next_value()?);
                        }
                        GeneratedField::Body => {
                            if body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("body"));
                            }
                            body__ = Some(map.next_value()?);
                        }
                        GeneratedField::ResponseBody => {
                            if response_body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseBody"));
                            }
                            response_body__ = Some(map.next_value()?);
                        }
                        GeneratedField::AdditionalBindings => {
                            if additional_bindings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("additionalBindings"));
                            }
                            additional_bindings__ = Some(map.next_value()?);
                        }
                        GeneratedField::Get => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("get"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Get);
                        }
                        GeneratedField::Put => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("put"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Put);
                        }
                        GeneratedField::Post => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("post"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Post);
                        }
                        GeneratedField::Delete => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("delete"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Delete);
                        }
                        GeneratedField::Patch => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("patch"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Patch);
                        }
                        GeneratedField::Custom => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("custom"));
                            }
                            pattern__ = map.next_value::<::std::option::Option<_>>()?.map(http_rule::Pattern::Custom)
;
                        }
                    }
                }
                Ok(HttpRule {
                    selector: selector__.unwrap_or_default(),
                    body: body__.unwrap_or_default(),
                    response_body: response_body__.unwrap_or_default(),
                    additional_bindings: additional_bindings__.unwrap_or_default(),
                    pattern: pattern__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.HttpRule", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for JavaSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.library_package.is_empty() {
            len += 1;
        }
        if !self.service_class_names.is_empty() {
            len += 1;
        }
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.JavaSettings", len)?;
        if !self.library_package.is_empty() {
            struct_ser.serialize_field("libraryPackage", &self.library_package)?;
        }
        if !self.service_class_names.is_empty() {
            struct_ser.serialize_field("serviceClassNames", &self.service_class_names)?;
        }
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for JavaSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "library_package",
            "libraryPackage",
            "service_class_names",
            "serviceClassNames",
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LibraryPackage,
            ServiceClassNames,
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "libraryPackage" | "library_package" => Ok(GeneratedField::LibraryPackage),
                            "serviceClassNames" | "service_class_names" => Ok(GeneratedField::ServiceClassNames),
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = JavaSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.JavaSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<JavaSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut library_package__ = None;
                let mut service_class_names__ = None;
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::LibraryPackage => {
                            if library_package__.is_some() {
                                return Err(serde::de::Error::duplicate_field("libraryPackage"));
                            }
                            library_package__ = Some(map.next_value()?);
                        }
                        GeneratedField::ServiceClassNames => {
                            if service_class_names__.is_some() {
                                return Err(serde::de::Error::duplicate_field("serviceClassNames"));
                            }
                            service_class_names__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(JavaSettings {
                    library_package: library_package__.unwrap_or_default(),
                    service_class_names: service_class_names__.unwrap_or_default(),
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.JavaSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LaunchStage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "LAUNCH_STAGE_UNSPECIFIED",
            Self::Unimplemented => "UNIMPLEMENTED",
            Self::Prelaunch => "PRELAUNCH",
            Self::EarlyAccess => "EARLY_ACCESS",
            Self::Alpha => "ALPHA",
            Self::Beta => "BETA",
            Self::Ga => "GA",
            Self::Deprecated => "DEPRECATED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for LaunchStage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "LAUNCH_STAGE_UNSPECIFIED",
            "UNIMPLEMENTED",
            "PRELAUNCH",
            "EARLY_ACCESS",
            "ALPHA",
            "BETA",
            "GA",
            "DEPRECATED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LaunchStage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(LaunchStage::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(LaunchStage::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "LAUNCH_STAGE_UNSPECIFIED" => Ok(LaunchStage::Unspecified),
                    "UNIMPLEMENTED" => Ok(LaunchStage::Unimplemented),
                    "PRELAUNCH" => Ok(LaunchStage::Prelaunch),
                    "EARLY_ACCESS" => Ok(LaunchStage::EarlyAccess),
                    "ALPHA" => Ok(LaunchStage::Alpha),
                    "BETA" => Ok(LaunchStage::Beta),
                    "GA" => Ok(LaunchStage::Ga),
                    "DEPRECATED" => Ok(LaunchStage::Deprecated),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for MethodSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.selector.is_empty() {
            len += 1;
        }
        if self.long_running.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.MethodSettings", len)?;
        if !self.selector.is_empty() {
            struct_ser.serialize_field("selector", &self.selector)?;
        }
        if let Some(v) = self.long_running.as_ref() {
            struct_ser.serialize_field("longRunning", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MethodSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "selector",
            "long_running",
            "longRunning",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Selector,
            LongRunning,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "selector" => Ok(GeneratedField::Selector),
                            "longRunning" | "long_running" => Ok(GeneratedField::LongRunning),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MethodSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.MethodSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MethodSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut selector__ = None;
                let mut long_running__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Selector => {
                            if selector__.is_some() {
                                return Err(serde::de::Error::duplicate_field("selector"));
                            }
                            selector__ = Some(map.next_value()?);
                        }
                        GeneratedField::LongRunning => {
                            if long_running__.is_some() {
                                return Err(serde::de::Error::duplicate_field("longRunning"));
                            }
                            long_running__ = map.next_value()?;
                        }
                    }
                }
                Ok(MethodSettings {
                    selector: selector__.unwrap_or_default(),
                    long_running: long_running__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.MethodSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for method_settings::LongRunning {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.initial_poll_delay.is_some() {
            len += 1;
        }
        if self.poll_delay_multiplier != 0. {
            len += 1;
        }
        if self.max_poll_delay.is_some() {
            len += 1;
        }
        if self.total_poll_timeout.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.MethodSettings.LongRunning", len)?;
        if let Some(v) = self.initial_poll_delay.as_ref() {
            struct_ser.serialize_field("initialPollDelay", v)?;
        }
        if self.poll_delay_multiplier != 0. {
            struct_ser.serialize_field("pollDelayMultiplier", &self.poll_delay_multiplier)?;
        }
        if let Some(v) = self.max_poll_delay.as_ref() {
            struct_ser.serialize_field("maxPollDelay", v)?;
        }
        if let Some(v) = self.total_poll_timeout.as_ref() {
            struct_ser.serialize_field("totalPollTimeout", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for method_settings::LongRunning {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "initial_poll_delay",
            "initialPollDelay",
            "poll_delay_multiplier",
            "pollDelayMultiplier",
            "max_poll_delay",
            "maxPollDelay",
            "total_poll_timeout",
            "totalPollTimeout",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            InitialPollDelay,
            PollDelayMultiplier,
            MaxPollDelay,
            TotalPollTimeout,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "initialPollDelay" | "initial_poll_delay" => Ok(GeneratedField::InitialPollDelay),
                            "pollDelayMultiplier" | "poll_delay_multiplier" => Ok(GeneratedField::PollDelayMultiplier),
                            "maxPollDelay" | "max_poll_delay" => Ok(GeneratedField::MaxPollDelay),
                            "totalPollTimeout" | "total_poll_timeout" => Ok(GeneratedField::TotalPollTimeout),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = method_settings::LongRunning;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.MethodSettings.LongRunning")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<method_settings::LongRunning, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut initial_poll_delay__ = None;
                let mut poll_delay_multiplier__ = None;
                let mut max_poll_delay__ = None;
                let mut total_poll_timeout__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::InitialPollDelay => {
                            if initial_poll_delay__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initialPollDelay"));
                            }
                            initial_poll_delay__ = map.next_value()?;
                        }
                        GeneratedField::PollDelayMultiplier => {
                            if poll_delay_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pollDelayMultiplier"));
                            }
                            poll_delay_multiplier__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MaxPollDelay => {
                            if max_poll_delay__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxPollDelay"));
                            }
                            max_poll_delay__ = map.next_value()?;
                        }
                        GeneratedField::TotalPollTimeout => {
                            if total_poll_timeout__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalPollTimeout"));
                            }
                            total_poll_timeout__ = map.next_value()?;
                        }
                    }
                }
                Ok(method_settings::LongRunning {
                    initial_poll_delay: initial_poll_delay__,
                    poll_delay_multiplier: poll_delay_multiplier__.unwrap_or_default(),
                    max_poll_delay: max_poll_delay__,
                    total_poll_timeout: total_poll_timeout__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.MethodSettings.LongRunning", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NodeSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.NodeSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NodeSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NodeSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.NodeSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<NodeSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(NodeSettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.NodeSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PhpSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.PhpSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PhpSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PhpSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.PhpSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PhpSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(PhpSettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.PhpSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Publishing {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.method_settings.is_empty() {
            len += 1;
        }
        if !self.new_issue_uri.is_empty() {
            len += 1;
        }
        if !self.documentation_uri.is_empty() {
            len += 1;
        }
        if !self.api_short_name.is_empty() {
            len += 1;
        }
        if !self.github_label.is_empty() {
            len += 1;
        }
        if !self.codeowner_github_teams.is_empty() {
            len += 1;
        }
        if !self.doc_tag_prefix.is_empty() {
            len += 1;
        }
        if self.organization != 0 {
            len += 1;
        }
        if !self.library_settings.is_empty() {
            len += 1;
        }
        if !self.proto_reference_documentation_uri.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.Publishing", len)?;
        if !self.method_settings.is_empty() {
            struct_ser.serialize_field("methodSettings", &self.method_settings)?;
        }
        if !self.new_issue_uri.is_empty() {
            struct_ser.serialize_field("newIssueUri", &self.new_issue_uri)?;
        }
        if !self.documentation_uri.is_empty() {
            struct_ser.serialize_field("documentationUri", &self.documentation_uri)?;
        }
        if !self.api_short_name.is_empty() {
            struct_ser.serialize_field("apiShortName", &self.api_short_name)?;
        }
        if !self.github_label.is_empty() {
            struct_ser.serialize_field("githubLabel", &self.github_label)?;
        }
        if !self.codeowner_github_teams.is_empty() {
            struct_ser.serialize_field("codeownerGithubTeams", &self.codeowner_github_teams)?;
        }
        if !self.doc_tag_prefix.is_empty() {
            struct_ser.serialize_field("docTagPrefix", &self.doc_tag_prefix)?;
        }
        if self.organization != 0 {
            let v = ClientLibraryOrganization::from_i32(self.organization)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.organization)))?;
            struct_ser.serialize_field("organization", &v)?;
        }
        if !self.library_settings.is_empty() {
            struct_ser.serialize_field("librarySettings", &self.library_settings)?;
        }
        if !self.proto_reference_documentation_uri.is_empty() {
            struct_ser.serialize_field("protoReferenceDocumentationUri", &self.proto_reference_documentation_uri)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Publishing {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "method_settings",
            "methodSettings",
            "new_issue_uri",
            "newIssueUri",
            "documentation_uri",
            "documentationUri",
            "api_short_name",
            "apiShortName",
            "github_label",
            "githubLabel",
            "codeowner_github_teams",
            "codeownerGithubTeams",
            "doc_tag_prefix",
            "docTagPrefix",
            "organization",
            "library_settings",
            "librarySettings",
            "proto_reference_documentation_uri",
            "protoReferenceDocumentationUri",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MethodSettings,
            NewIssueUri,
            DocumentationUri,
            ApiShortName,
            GithubLabel,
            CodeownerGithubTeams,
            DocTagPrefix,
            Organization,
            LibrarySettings,
            ProtoReferenceDocumentationUri,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "methodSettings" | "method_settings" => Ok(GeneratedField::MethodSettings),
                            "newIssueUri" | "new_issue_uri" => Ok(GeneratedField::NewIssueUri),
                            "documentationUri" | "documentation_uri" => Ok(GeneratedField::DocumentationUri),
                            "apiShortName" | "api_short_name" => Ok(GeneratedField::ApiShortName),
                            "githubLabel" | "github_label" => Ok(GeneratedField::GithubLabel),
                            "codeownerGithubTeams" | "codeowner_github_teams" => Ok(GeneratedField::CodeownerGithubTeams),
                            "docTagPrefix" | "doc_tag_prefix" => Ok(GeneratedField::DocTagPrefix),
                            "organization" => Ok(GeneratedField::Organization),
                            "librarySettings" | "library_settings" => Ok(GeneratedField::LibrarySettings),
                            "protoReferenceDocumentationUri" | "proto_reference_documentation_uri" => Ok(GeneratedField::ProtoReferenceDocumentationUri),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Publishing;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.Publishing")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Publishing, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut method_settings__ = None;
                let mut new_issue_uri__ = None;
                let mut documentation_uri__ = None;
                let mut api_short_name__ = None;
                let mut github_label__ = None;
                let mut codeowner_github_teams__ = None;
                let mut doc_tag_prefix__ = None;
                let mut organization__ = None;
                let mut library_settings__ = None;
                let mut proto_reference_documentation_uri__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::MethodSettings => {
                            if method_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("methodSettings"));
                            }
                            method_settings__ = Some(map.next_value()?);
                        }
                        GeneratedField::NewIssueUri => {
                            if new_issue_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newIssueUri"));
                            }
                            new_issue_uri__ = Some(map.next_value()?);
                        }
                        GeneratedField::DocumentationUri => {
                            if documentation_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("documentationUri"));
                            }
                            documentation_uri__ = Some(map.next_value()?);
                        }
                        GeneratedField::ApiShortName => {
                            if api_short_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("apiShortName"));
                            }
                            api_short_name__ = Some(map.next_value()?);
                        }
                        GeneratedField::GithubLabel => {
                            if github_label__.is_some() {
                                return Err(serde::de::Error::duplicate_field("githubLabel"));
                            }
                            github_label__ = Some(map.next_value()?);
                        }
                        GeneratedField::CodeownerGithubTeams => {
                            if codeowner_github_teams__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codeownerGithubTeams"));
                            }
                            codeowner_github_teams__ = Some(map.next_value()?);
                        }
                        GeneratedField::DocTagPrefix => {
                            if doc_tag_prefix__.is_some() {
                                return Err(serde::de::Error::duplicate_field("docTagPrefix"));
                            }
                            doc_tag_prefix__ = Some(map.next_value()?);
                        }
                        GeneratedField::Organization => {
                            if organization__.is_some() {
                                return Err(serde::de::Error::duplicate_field("organization"));
                            }
                            organization__ = Some(map.next_value::<ClientLibraryOrganization>()? as i32);
                        }
                        GeneratedField::LibrarySettings => {
                            if library_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("librarySettings"));
                            }
                            library_settings__ = Some(map.next_value()?);
                        }
                        GeneratedField::ProtoReferenceDocumentationUri => {
                            if proto_reference_documentation_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("protoReferenceDocumentationUri"));
                            }
                            proto_reference_documentation_uri__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(Publishing {
                    method_settings: method_settings__.unwrap_or_default(),
                    new_issue_uri: new_issue_uri__.unwrap_or_default(),
                    documentation_uri: documentation_uri__.unwrap_or_default(),
                    api_short_name: api_short_name__.unwrap_or_default(),
                    github_label: github_label__.unwrap_or_default(),
                    codeowner_github_teams: codeowner_github_teams__.unwrap_or_default(),
                    doc_tag_prefix: doc_tag_prefix__.unwrap_or_default(),
                    organization: organization__.unwrap_or_default(),
                    library_settings: library_settings__.unwrap_or_default(),
                    proto_reference_documentation_uri: proto_reference_documentation_uri__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.Publishing", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PythonSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.PythonSettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PythonSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PythonSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.PythonSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PythonSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(PythonSettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.PythonSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ResourceDescriptor {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.r#type.is_empty() {
            len += 1;
        }
        if !self.pattern.is_empty() {
            len += 1;
        }
        if !self.name_field.is_empty() {
            len += 1;
        }
        if self.history != 0 {
            len += 1;
        }
        if !self.plural.is_empty() {
            len += 1;
        }
        if !self.singular.is_empty() {
            len += 1;
        }
        if !self.style.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.ResourceDescriptor", len)?;
        if !self.r#type.is_empty() {
            struct_ser.serialize_field("type", &self.r#type)?;
        }
        if !self.pattern.is_empty() {
            struct_ser.serialize_field("pattern", &self.pattern)?;
        }
        if !self.name_field.is_empty() {
            struct_ser.serialize_field("nameField", &self.name_field)?;
        }
        if self.history != 0 {
            let v = resource_descriptor::History::from_i32(self.history)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.history)))?;
            struct_ser.serialize_field("history", &v)?;
        }
        if !self.plural.is_empty() {
            struct_ser.serialize_field("plural", &self.plural)?;
        }
        if !self.singular.is_empty() {
            struct_ser.serialize_field("singular", &self.singular)?;
        }
        if !self.style.is_empty() {
            let v = self.style.iter().cloned().map(|v| {
                resource_descriptor::Style::from_i32(v)
                    .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("style", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ResourceDescriptor {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "pattern",
            "name_field",
            "nameField",
            "history",
            "plural",
            "singular",
            "style",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Pattern,
            NameField,
            History,
            Plural,
            Singular,
            Style,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "pattern" => Ok(GeneratedField::Pattern),
                            "nameField" | "name_field" => Ok(GeneratedField::NameField),
                            "history" => Ok(GeneratedField::History),
                            "plural" => Ok(GeneratedField::Plural),
                            "singular" => Ok(GeneratedField::Singular),
                            "style" => Ok(GeneratedField::Style),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ResourceDescriptor;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.ResourceDescriptor")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ResourceDescriptor, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut pattern__ = None;
                let mut name_field__ = None;
                let mut history__ = None;
                let mut plural__ = None;
                let mut singular__ = None;
                let mut style__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
                        }
                        GeneratedField::Pattern => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pattern"));
                            }
                            pattern__ = Some(map.next_value()?);
                        }
                        GeneratedField::NameField => {
                            if name_field__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nameField"));
                            }
                            name_field__ = Some(map.next_value()?);
                        }
                        GeneratedField::History => {
                            if history__.is_some() {
                                return Err(serde::de::Error::duplicate_field("history"));
                            }
                            history__ = Some(map.next_value::<resource_descriptor::History>()? as i32);
                        }
                        GeneratedField::Plural => {
                            if plural__.is_some() {
                                return Err(serde::de::Error::duplicate_field("plural"));
                            }
                            plural__ = Some(map.next_value()?);
                        }
                        GeneratedField::Singular => {
                            if singular__.is_some() {
                                return Err(serde::de::Error::duplicate_field("singular"));
                            }
                            singular__ = Some(map.next_value()?);
                        }
                        GeneratedField::Style => {
                            if style__.is_some() {
                                return Err(serde::de::Error::duplicate_field("style"));
                            }
                            style__ = Some(map.next_value::<Vec<resource_descriptor::Style>>()?.into_iter().map(|x| x as i32).collect());
                        }
                    }
                }
                Ok(ResourceDescriptor {
                    r#type: r#type__.unwrap_or_default(),
                    pattern: pattern__.unwrap_or_default(),
                    name_field: name_field__.unwrap_or_default(),
                    history: history__.unwrap_or_default(),
                    plural: plural__.unwrap_or_default(),
                    singular: singular__.unwrap_or_default(),
                    style: style__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.ResourceDescriptor", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for resource_descriptor::History {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "HISTORY_UNSPECIFIED",
            Self::OriginallySinglePattern => "ORIGINALLY_SINGLE_PATTERN",
            Self::FutureMultiPattern => "FUTURE_MULTI_PATTERN",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for resource_descriptor::History {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "HISTORY_UNSPECIFIED",
            "ORIGINALLY_SINGLE_PATTERN",
            "FUTURE_MULTI_PATTERN",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = resource_descriptor::History;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(resource_descriptor::History::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(resource_descriptor::History::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "HISTORY_UNSPECIFIED" => Ok(resource_descriptor::History::Unspecified),
                    "ORIGINALLY_SINGLE_PATTERN" => Ok(resource_descriptor::History::OriginallySinglePattern),
                    "FUTURE_MULTI_PATTERN" => Ok(resource_descriptor::History::FutureMultiPattern),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for resource_descriptor::Style {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STYLE_UNSPECIFIED",
            Self::DeclarativeFriendly => "DECLARATIVE_FRIENDLY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for resource_descriptor::Style {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STYLE_UNSPECIFIED",
            "DECLARATIVE_FRIENDLY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = resource_descriptor::Style;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(resource_descriptor::Style::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(resource_descriptor::Style::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STYLE_UNSPECIFIED" => Ok(resource_descriptor::Style::Unspecified),
                    "DECLARATIVE_FRIENDLY" => Ok(resource_descriptor::Style::DeclarativeFriendly),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ResourceReference {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.r#type.is_empty() {
            len += 1;
        }
        if !self.child_type.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.ResourceReference", len)?;
        if !self.r#type.is_empty() {
            struct_ser.serialize_field("type", &self.r#type)?;
        }
        if !self.child_type.is_empty() {
            struct_ser.serialize_field("childType", &self.child_type)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ResourceReference {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "child_type",
            "childType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            ChildType,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "childType" | "child_type" => Ok(GeneratedField::ChildType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ResourceReference;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.ResourceReference")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ResourceReference, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut child_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
                        }
                        GeneratedField::ChildType => {
                            if child_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("childType"));
                            }
                            child_type__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ResourceReference {
                    r#type: r#type__.unwrap_or_default(),
                    child_type: child_type__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.api.ResourceReference", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RubySettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.api.RubySettings", len)?;
        if let Some(v) = self.common.as_ref() {
            struct_ser.serialize_field("common", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RubySettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "common",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Common,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "common" => Ok(GeneratedField::Common),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RubySettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.api.RubySettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<RubySettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut common__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Common => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("common"));
                            }
                            common__ = map.next_value()?;
                        }
                    }
                }
                Ok(RubySettings {
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("google.api.RubySettings", FIELDS, GeneratedVisitor)
    }
}
