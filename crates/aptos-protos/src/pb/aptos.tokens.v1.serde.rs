// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for CollectionData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.creator_address.is_empty() {
            len += 1;
        }
        if !self.collection_name.is_empty() {
            len += 1;
        }
        if !self.description.is_empty() {
            len += 1;
        }
        if self.transaction_version != 0 {
            len += 1;
        }
        if !self.metadata_uri.is_empty() {
            len += 1;
        }
        if self.supply != 0 {
            len += 1;
        }
        if self.maximum != 0 {
            len += 1;
        }
        if self.maximum_mutable {
            len += 1;
        }
        if self.uri_mutable {
            len += 1;
        }
        if self.description_mutable {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.CollectionData", len)?;
        if !self.creator_address.is_empty() {
            struct_ser.serialize_field("creatorAddress", &self.creator_address)?;
        }
        if !self.collection_name.is_empty() {
            struct_ser.serialize_field("collectionName", &self.collection_name)?;
        }
        if !self.description.is_empty() {
            struct_ser.serialize_field("description", &self.description)?;
        }
        if self.transaction_version != 0 {
            struct_ser.serialize_field(
                "transactionVersion",
                ToString::to_string(&self.transaction_version).as_str(),
            )?;
        }
        if !self.metadata_uri.is_empty() {
            struct_ser.serialize_field("metadataUri", &self.metadata_uri)?;
        }
        if self.supply != 0 {
            struct_ser.serialize_field("supply", ToString::to_string(&self.supply).as_str())?;
        }
        if self.maximum != 0 {
            struct_ser.serialize_field("maximum", ToString::to_string(&self.maximum).as_str())?;
        }
        if self.maximum_mutable {
            struct_ser.serialize_field("maximumMutable", &self.maximum_mutable)?;
        }
        if self.uri_mutable {
            struct_ser.serialize_field("uriMutable", &self.uri_mutable)?;
        }
        if self.description_mutable {
            struct_ser.serialize_field("descriptionMutable", &self.description_mutable)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CollectionData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "creatorAddress",
            "collectionName",
            "description",
            "transactionVersion",
            "metadataUri",
            "supply",
            "maximum",
            "maximumMutable",
            "uriMutable",
            "descriptionMutable",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CreatorAddress,
            CollectionName,
            Description,
            TransactionVersion,
            MetadataUri,
            Supply,
            Maximum,
            MaximumMutable,
            UriMutable,
            DescriptionMutable,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "creatorAddress" => Ok(GeneratedField::CreatorAddress),
                            "collectionName" => Ok(GeneratedField::CollectionName),
                            "description" => Ok(GeneratedField::Description),
                            "transactionVersion" => Ok(GeneratedField::TransactionVersion),
                            "metadataUri" => Ok(GeneratedField::MetadataUri),
                            "supply" => Ok(GeneratedField::Supply),
                            "maximum" => Ok(GeneratedField::Maximum),
                            "maximumMutable" => Ok(GeneratedField::MaximumMutable),
                            "uriMutable" => Ok(GeneratedField::UriMutable),
                            "descriptionMutable" => Ok(GeneratedField::DescriptionMutable),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CollectionData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.CollectionData")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CollectionData, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut creator_address__ = None;
                let mut collection_name__ = None;
                let mut description__ = None;
                let mut transaction_version__ = None;
                let mut metadata_uri__ = None;
                let mut supply__ = None;
                let mut maximum__ = None;
                let mut maximum_mutable__ = None;
                let mut uri_mutable__ = None;
                let mut description_mutable__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::CreatorAddress => {
                            if creator_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creatorAddress"));
                            }
                            creator_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::CollectionName => {
                            if collection_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("collectionName"));
                            }
                            collection_name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Description => {
                            if description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("description"));
                            }
                            description__ = Some(map.next_value()?);
                        }
                        GeneratedField::TransactionVersion => {
                            if transaction_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionVersion",
                                ));
                            }
                            transaction_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MetadataUri => {
                            if metadata_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataUri"));
                            }
                            metadata_uri__ = Some(map.next_value()?);
                        }
                        GeneratedField::Supply => {
                            if supply__.is_some() {
                                return Err(serde::de::Error::duplicate_field("supply"));
                            }
                            supply__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Maximum => {
                            if maximum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximum"));
                            }
                            maximum__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MaximumMutable => {
                            if maximum_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximumMutable"));
                            }
                            maximum_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::UriMutable => {
                            if uri_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uriMutable"));
                            }
                            uri_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::DescriptionMutable => {
                            if description_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "descriptionMutable",
                                ));
                            }
                            description_mutable__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(CollectionData {
                    creator_address: creator_address__.unwrap_or_default(),
                    collection_name: collection_name__.unwrap_or_default(),
                    description: description__.unwrap_or_default(),
                    transaction_version: transaction_version__.unwrap_or_default(),
                    metadata_uri: metadata_uri__.unwrap_or_default(),
                    supply: supply__.unwrap_or_default(),
                    maximum: maximum__.unwrap_or_default(),
                    maximum_mutable: maximum_mutable__.unwrap_or_default(),
                    uri_mutable: uri_mutable__.unwrap_or_default(),
                    description_mutable: description_mutable__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.CollectionData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Token {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.token_id.is_some() {
            len += 1;
        }
        if self.transaction_version != 0 {
            len += 1;
        }
        if !self.token_properties.is_empty() {
            len += 1;
        }
        if self.amount != 0 {
            len += 1;
        }
        if self.owner_address.is_some() {
            len += 1;
        }
        if !self.table_handle.is_empty() {
            len += 1;
        }
        if self.table_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.Token", len)?;
        if let Some(v) = self.token_id.as_ref() {
            struct_ser.serialize_field("tokenId", v)?;
        }
        if self.transaction_version != 0 {
            struct_ser.serialize_field(
                "transactionVersion",
                ToString::to_string(&self.transaction_version).as_str(),
            )?;
        }
        if !self.token_properties.is_empty() {
            struct_ser.serialize_field("tokenProperties", &self.token_properties)?;
        }
        if self.amount != 0 {
            struct_ser.serialize_field("amount", ToString::to_string(&self.amount).as_str())?;
        }
        if let Some(v) = self.owner_address.as_ref() {
            struct_ser.serialize_field("ownerAddress", v)?;
        }
        if !self.table_handle.is_empty() {
            struct_ser.serialize_field("tableHandle", &self.table_handle)?;
        }
        if let Some(v) = self.table_type.as_ref() {
            struct_ser.serialize_field("tableType", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Token {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tokenId",
            "transactionVersion",
            "tokenProperties",
            "amount",
            "ownerAddress",
            "tableHandle",
            "tableType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TokenId,
            TransactionVersion,
            TokenProperties,
            Amount,
            OwnerAddress,
            TableHandle,
            TableType,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tokenId" => Ok(GeneratedField::TokenId),
                            "transactionVersion" => Ok(GeneratedField::TransactionVersion),
                            "tokenProperties" => Ok(GeneratedField::TokenProperties),
                            "amount" => Ok(GeneratedField::Amount),
                            "ownerAddress" => Ok(GeneratedField::OwnerAddress),
                            "tableHandle" => Ok(GeneratedField::TableHandle),
                            "tableType" => Ok(GeneratedField::TableType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Token;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.Token")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Token, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut token_id__ = None;
                let mut transaction_version__ = None;
                let mut token_properties__ = None;
                let mut amount__ = None;
                let mut owner_address__ = None;
                let mut table_handle__ = None;
                let mut table_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TokenId => {
                            if token_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenId"));
                            }
                            token_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::TransactionVersion => {
                            if transaction_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionVersion",
                                ));
                            }
                            transaction_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::TokenProperties => {
                            if token_properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenProperties"));
                            }
                            token_properties__ = Some(map.next_value()?);
                        }
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::OwnerAddress => {
                            if owner_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ownerAddress"));
                            }
                            owner_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::TableHandle => {
                            if table_handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tableHandle"));
                            }
                            table_handle__ = Some(map.next_value()?);
                        }
                        GeneratedField::TableType => {
                            if table_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tableType"));
                            }
                            table_type__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(Token {
                    token_id: token_id__,
                    transaction_version: transaction_version__.unwrap_or_default(),
                    token_properties: token_properties__.unwrap_or_default(),
                    amount: amount__.unwrap_or_default(),
                    owner_address: owner_address__,
                    table_handle: table_handle__.unwrap_or_default(),
                    table_type: table_type__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.Token", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.token_data_id.is_some() {
            len += 1;
        }
        if self.transaction_version != 0 {
            len += 1;
        }
        if self.maximum != 0 {
            len += 1;
        }
        if self.supply != 0 {
            len += 1;
        }
        if self.largest_property_version != 0 {
            len += 1;
        }
        if !self.metadata_uri.is_empty() {
            len += 1;
        }
        if !self.payee_address.is_empty() {
            len += 1;
        }
        if self.royalty_points_numerator != 0 {
            len += 1;
        }
        if self.royalty_points_denominator != 0 {
            len += 1;
        }
        if self.maximum_mutable {
            len += 1;
        }
        if self.uri_mutable {
            len += 1;
        }
        if self.description_mutable {
            len += 1;
        }
        if self.properties_mutable {
            len += 1;
        }
        if self.royalty_mutable {
            len += 1;
        }
        if !self.default_properties.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.TokenData", len)?;
        if let Some(v) = self.token_data_id.as_ref() {
            struct_ser.serialize_field("tokenDataId", v)?;
        }
        if self.transaction_version != 0 {
            struct_ser.serialize_field(
                "transactionVersion",
                ToString::to_string(&self.transaction_version).as_str(),
            )?;
        }
        if self.maximum != 0 {
            struct_ser.serialize_field("maximum", ToString::to_string(&self.maximum).as_str())?;
        }
        if self.supply != 0 {
            struct_ser.serialize_field("supply", ToString::to_string(&self.supply).as_str())?;
        }
        if self.largest_property_version != 0 {
            struct_ser.serialize_field(
                "largestPropertyVersion",
                ToString::to_string(&self.largest_property_version).as_str(),
            )?;
        }
        if !self.metadata_uri.is_empty() {
            struct_ser.serialize_field("metadataUri", &self.metadata_uri)?;
        }
        if !self.payee_address.is_empty() {
            struct_ser.serialize_field("payeeAddress", &self.payee_address)?;
        }
        if self.royalty_points_numerator != 0 {
            struct_ser.serialize_field(
                "royaltyPointsNumerator",
                ToString::to_string(&self.royalty_points_numerator).as_str(),
            )?;
        }
        if self.royalty_points_denominator != 0 {
            struct_ser.serialize_field(
                "royaltyPointsDenominator",
                ToString::to_string(&self.royalty_points_denominator).as_str(),
            )?;
        }
        if self.maximum_mutable {
            struct_ser.serialize_field("maximumMutable", &self.maximum_mutable)?;
        }
        if self.uri_mutable {
            struct_ser.serialize_field("uriMutable", &self.uri_mutable)?;
        }
        if self.description_mutable {
            struct_ser.serialize_field("descriptionMutable", &self.description_mutable)?;
        }
        if self.properties_mutable {
            struct_ser.serialize_field("propertiesMutable", &self.properties_mutable)?;
        }
        if self.royalty_mutable {
            struct_ser.serialize_field("royaltyMutable", &self.royalty_mutable)?;
        }
        if !self.default_properties.is_empty() {
            struct_ser.serialize_field("defaultProperties", &self.default_properties)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tokenDataId",
            "transactionVersion",
            "maximum",
            "supply",
            "largestPropertyVersion",
            "metadataUri",
            "payeeAddress",
            "royaltyPointsNumerator",
            "royaltyPointsDenominator",
            "maximumMutable",
            "uriMutable",
            "descriptionMutable",
            "propertiesMutable",
            "royaltyMutable",
            "defaultProperties",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TokenDataId,
            TransactionVersion,
            Maximum,
            Supply,
            LargestPropertyVersion,
            MetadataUri,
            PayeeAddress,
            RoyaltyPointsNumerator,
            RoyaltyPointsDenominator,
            MaximumMutable,
            UriMutable,
            DescriptionMutable,
            PropertiesMutable,
            RoyaltyMutable,
            DefaultProperties,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tokenDataId" => Ok(GeneratedField::TokenDataId),
                            "transactionVersion" => Ok(GeneratedField::TransactionVersion),
                            "maximum" => Ok(GeneratedField::Maximum),
                            "supply" => Ok(GeneratedField::Supply),
                            "largestPropertyVersion" => Ok(GeneratedField::LargestPropertyVersion),
                            "metadataUri" => Ok(GeneratedField::MetadataUri),
                            "payeeAddress" => Ok(GeneratedField::PayeeAddress),
                            "royaltyPointsNumerator" => Ok(GeneratedField::RoyaltyPointsNumerator),
                            "royaltyPointsDenominator" => {
                                Ok(GeneratedField::RoyaltyPointsDenominator)
                            }
                            "maximumMutable" => Ok(GeneratedField::MaximumMutable),
                            "uriMutable" => Ok(GeneratedField::UriMutable),
                            "descriptionMutable" => Ok(GeneratedField::DescriptionMutable),
                            "propertiesMutable" => Ok(GeneratedField::PropertiesMutable),
                            "royaltyMutable" => Ok(GeneratedField::RoyaltyMutable),
                            "defaultProperties" => Ok(GeneratedField::DefaultProperties),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.TokenData")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TokenData, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut token_data_id__ = None;
                let mut transaction_version__ = None;
                let mut maximum__ = None;
                let mut supply__ = None;
                let mut largest_property_version__ = None;
                let mut metadata_uri__ = None;
                let mut payee_address__ = None;
                let mut royalty_points_numerator__ = None;
                let mut royalty_points_denominator__ = None;
                let mut maximum_mutable__ = None;
                let mut uri_mutable__ = None;
                let mut description_mutable__ = None;
                let mut properties_mutable__ = None;
                let mut royalty_mutable__ = None;
                let mut default_properties__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TokenDataId => {
                            if token_data_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenDataId"));
                            }
                            token_data_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::TransactionVersion => {
                            if transaction_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionVersion",
                                ));
                            }
                            transaction_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Maximum => {
                            if maximum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximum"));
                            }
                            maximum__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Supply => {
                            if supply__.is_some() {
                                return Err(serde::de::Error::duplicate_field("supply"));
                            }
                            supply__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::LargestPropertyVersion => {
                            if largest_property_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "largestPropertyVersion",
                                ));
                            }
                            largest_property_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MetadataUri => {
                            if metadata_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataUri"));
                            }
                            metadata_uri__ = Some(map.next_value()?);
                        }
                        GeneratedField::PayeeAddress => {
                            if payee_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payeeAddress"));
                            }
                            payee_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::RoyaltyPointsNumerator => {
                            if royalty_points_numerator__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "royaltyPointsNumerator",
                                ));
                            }
                            royalty_points_numerator__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::RoyaltyPointsDenominator => {
                            if royalty_points_denominator__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "royaltyPointsDenominator",
                                ));
                            }
                            royalty_points_denominator__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MaximumMutable => {
                            if maximum_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximumMutable"));
                            }
                            maximum_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::UriMutable => {
                            if uri_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uriMutable"));
                            }
                            uri_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::DescriptionMutable => {
                            if description_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "descriptionMutable",
                                ));
                            }
                            description_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::PropertiesMutable => {
                            if properties_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("propertiesMutable"));
                            }
                            properties_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::RoyaltyMutable => {
                            if royalty_mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("royaltyMutable"));
                            }
                            royalty_mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::DefaultProperties => {
                            if default_properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("defaultProperties"));
                            }
                            default_properties__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TokenData {
                    token_data_id: token_data_id__,
                    transaction_version: transaction_version__.unwrap_or_default(),
                    maximum: maximum__.unwrap_or_default(),
                    supply: supply__.unwrap_or_default(),
                    largest_property_version: largest_property_version__.unwrap_or_default(),
                    metadata_uri: metadata_uri__.unwrap_or_default(),
                    payee_address: payee_address__.unwrap_or_default(),
                    royalty_points_numerator: royalty_points_numerator__.unwrap_or_default(),
                    royalty_points_denominator: royalty_points_denominator__.unwrap_or_default(),
                    maximum_mutable: maximum_mutable__.unwrap_or_default(),
                    uri_mutable: uri_mutable__.unwrap_or_default(),
                    description_mutable: description_mutable__.unwrap_or_default(),
                    properties_mutable: properties_mutable__.unwrap_or_default(),
                    royalty_mutable: royalty_mutable__.unwrap_or_default(),
                    default_properties: default_properties__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.TokenData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenDataId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.creator_address.is_empty() {
            len += 1;
        }
        if !self.collection_name.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.TokenDataId", len)?;
        if !self.creator_address.is_empty() {
            struct_ser.serialize_field("creatorAddress", &self.creator_address)?;
        }
        if !self.collection_name.is_empty() {
            struct_ser.serialize_field("collectionName", &self.collection_name)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenDataId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["creatorAddress", "collectionName", "name"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CreatorAddress,
            CollectionName,
            Name,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "creatorAddress" => Ok(GeneratedField::CreatorAddress),
                            "collectionName" => Ok(GeneratedField::CollectionName),
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenDataId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.TokenDataId")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TokenDataId, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut creator_address__ = None;
                let mut collection_name__ = None;
                let mut name__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::CreatorAddress => {
                            if creator_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creatorAddress"));
                            }
                            creator_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::CollectionName => {
                            if collection_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("collectionName"));
                            }
                            collection_name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TokenDataId {
                    creator_address: creator_address__.unwrap_or_default(),
                    collection_name: collection_name__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.TokenDataId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.token_data_id.is_some() {
            len += 1;
        }
        if self.property_version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.TokenId", len)?;
        if let Some(v) = self.token_data_id.as_ref() {
            struct_ser.serialize_field("tokenDataId", v)?;
        }
        if self.property_version != 0 {
            struct_ser.serialize_field(
                "propertyVersion",
                ToString::to_string(&self.property_version).as_str(),
            )?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["tokenDataId", "propertyVersion"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TokenDataId,
            PropertyVersion,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tokenDataId" => Ok(GeneratedField::TokenDataId),
                            "propertyVersion" => Ok(GeneratedField::PropertyVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.TokenId")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TokenId, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut token_data_id__ = None;
                let mut property_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TokenDataId => {
                            if token_data_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenDataId"));
                            }
                            token_data_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::PropertyVersion => {
                            if property_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("propertyVersion"));
                            }
                            property_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(TokenId {
                    token_data_id: token_data_id__,
                    property_version: property_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.TokenId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Tokens {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_height != 0 {
            len += 1;
        }
        if self.chain_id != 0 {
            len += 1;
        }
        if !self.tokens.is_empty() {
            len += 1;
        }
        if !self.token_datas.is_empty() {
            len += 1;
        }
        if !self.collection_datas.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.tokens.v1.Tokens", len)?;
        if self.block_height != 0 {
            struct_ser.serialize_field(
                "blockHeight",
                ToString::to_string(&self.block_height).as_str(),
            )?;
        }
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if !self.tokens.is_empty() {
            struct_ser.serialize_field("tokens", &self.tokens)?;
        }
        if !self.token_datas.is_empty() {
            struct_ser.serialize_field("tokenDatas", &self.token_datas)?;
        }
        if !self.collection_datas.is_empty() {
            struct_ser.serialize_field("collectionDatas", &self.collection_datas)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Tokens {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "blockHeight",
            "chainId",
            "tokens",
            "tokenDatas",
            "collectionDatas",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHeight,
            ChainId,
            Tokens,
            TokenDatas,
            CollectionDatas,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blockHeight" => Ok(GeneratedField::BlockHeight),
                            "chainId" => Ok(GeneratedField::ChainId),
                            "tokens" => Ok(GeneratedField::Tokens),
                            "tokenDatas" => Ok(GeneratedField::TokenDatas),
                            "collectionDatas" => Ok(GeneratedField::CollectionDatas),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Tokens;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.tokens.v1.Tokens")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Tokens, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut block_height__ = None;
                let mut chain_id__ = None;
                let mut tokens__ = None;
                let mut token_datas__ = None;
                let mut collection_datas__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Tokens => {
                            if tokens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokens"));
                            }
                            tokens__ = Some(map.next_value()?);
                        }
                        GeneratedField::TokenDatas => {
                            if token_datas__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenDatas"));
                            }
                            token_datas__ = Some(map.next_value()?);
                        }
                        GeneratedField::CollectionDatas => {
                            if collection_datas__.is_some() {
                                return Err(serde::de::Error::duplicate_field("collectionDatas"));
                            }
                            collection_datas__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(Tokens {
                    block_height: block_height__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                    tokens: tokens__.unwrap_or_default(),
                    token_datas: token_datas__.unwrap_or_default(),
                    collection_datas: collection_datas__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.tokens.v1.Tokens", FIELDS, GeneratedVisitor)
    }
}
