// Copyright © Aptos Foundation

/// A schema resource.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Schema {
    /// Required. Name of the schema.
    /// Format is `projects/{project}/schemas/{schema}`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// The type of the schema definition.
    #[prost(enumeration = "schema::Type", tag = "2")]
    pub r#type: i32,
    /// The definition of the schema. This should contain a string representing
    /// the full definition of the schema that is a valid schema definition of
    /// the type specified in `type`.
    #[prost(string, tag = "3")]
    pub definition: ::prost::alloc::string::String,
    /// Output only. Immutable. The revision ID of the schema.
    #[prost(string, tag = "4")]
    pub revision_id: ::prost::alloc::string::String,
    /// Output only. The timestamp that the revision was created.
    #[prost(message, optional, tag = "6")]
    pub revision_create_time: ::core::option::Option<::prost_types::Timestamp>,
}
/// Nested message and enum types in `Schema`.
pub mod schema {
    /// Possible schema definition types.
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Type {
        /// Default value. This value is unused.
        Unspecified = 0,
        /// A Protocol Buffer schema definition.
        ProtocolBuffer = 1,
        /// An Avro schema definition.
        Avro = 2,
    }
    impl Type {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Type::Unspecified => "TYPE_UNSPECIFIED",
                Type::ProtocolBuffer => "PROTOCOL_BUFFER",
                Type::Avro => "AVRO",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "TYPE_UNSPECIFIED" => Some(Self::Unspecified),
                "PROTOCOL_BUFFER" => Some(Self::ProtocolBuffer),
                "AVRO" => Some(Self::Avro),
                _ => None,
            }
        }
    }
}
/// Request for the CreateSchema method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateSchemaRequest {
    /// Required. The name of the project in which to create the schema.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub parent: ::prost::alloc::string::String,
    /// Required. The schema object to create.
    ///
    /// This schema's `name` parameter is ignored. The schema object returned
    /// by CreateSchema will have a `name` made using the given `parent` and
    /// `schema_id`.
    #[prost(message, optional, tag = "2")]
    pub schema: ::core::option::Option<Schema>,
    /// The ID to use for the schema, which will become the final component of
    /// the schema's resource name.
    ///
    /// See <https://cloud.google.com/pubsub/docs/admin#resource_names> for resource
    /// name constraints.
    #[prost(string, tag = "3")]
    pub schema_id: ::prost::alloc::string::String,
}
/// Request for the GetSchema method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSchemaRequest {
    /// Required. The name of the schema to get.
    /// Format is `projects/{project}/schemas/{schema}`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// The set of fields to return in the response. If not set, returns a Schema
    /// with all fields filled out. Set to `BASIC` to omit the `definition`.
    #[prost(enumeration = "SchemaView", tag = "2")]
    pub view: i32,
}
/// Request for the `ListSchemas` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSchemasRequest {
    /// Required. The name of the project in which to list schemas.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub parent: ::prost::alloc::string::String,
    /// The set of Schema fields to return in the response. If not set, returns
    /// Schemas with `name` and `type`, but not `definition`. Set to `FULL` to
    /// retrieve all fields.
    #[prost(enumeration = "SchemaView", tag = "2")]
    pub view: i32,
    /// Maximum number of schemas to return.
    #[prost(int32, tag = "3")]
    pub page_size: i32,
    /// The value returned by the last `ListSchemasResponse`; indicates that
    /// this is a continuation of a prior `ListSchemas` call, and that the
    /// system should return the next page of data.
    #[prost(string, tag = "4")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListSchemas` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSchemasResponse {
    /// The resulting schemas.
    #[prost(message, repeated, tag = "1")]
    pub schemas: ::prost::alloc::vec::Vec<Schema>,
    /// If not empty, indicates that there may be more schemas that match the
    /// request; this value should be passed in a new `ListSchemasRequest`.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the `ListSchemaRevisions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSchemaRevisionsRequest {
    /// Required. The name of the schema to list revisions for.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// The set of Schema fields to return in the response. If not set, returns
    /// Schemas with `name` and `type`, but not `definition`. Set to `FULL` to
    /// retrieve all fields.
    #[prost(enumeration = "SchemaView", tag = "2")]
    pub view: i32,
    /// The maximum number of revisions to return per page.
    #[prost(int32, tag = "3")]
    pub page_size: i32,
    /// The page token, received from a previous ListSchemaRevisions call.
    /// Provide this to retrieve the subsequent page.
    #[prost(string, tag = "4")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListSchemaRevisions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSchemaRevisionsResponse {
    /// The revisions of the schema.
    #[prost(message, repeated, tag = "1")]
    pub schemas: ::prost::alloc::vec::Vec<Schema>,
    /// A token that can be sent as `page_token` to retrieve the next page.
    /// If this field is empty, there are no subsequent pages.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for CommitSchema method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitSchemaRequest {
    /// Required. The name of the schema we are revising.
    /// Format is `projects/{project}/schemas/{schema}`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Required. The schema revision to commit.
    #[prost(message, optional, tag = "2")]
    pub schema: ::core::option::Option<Schema>,
}
/// Request for the `RollbackSchema` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollbackSchemaRequest {
    /// Required. The schema being rolled back with revision id.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Required. The revision ID to roll back to.
    /// It must be a revision of the same schema.
    ///
    ///    Example: c7cfa2a8
    #[prost(string, tag = "2")]
    pub revision_id: ::prost::alloc::string::String,
}
/// Request for the `DeleteSchemaRevision` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteSchemaRevisionRequest {
    /// Required. The name of the schema revision to be deleted, with a revision ID
    /// explicitly included.
    ///
    /// Example: `projects/123/schemas/my-schema@c7cfa2a8`
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Optional. This field is deprecated and should not be used for specifying
    /// the revision ID. The revision ID should be specified via the `name`
    /// parameter.
    #[deprecated]
    #[prost(string, tag = "2")]
    pub revision_id: ::prost::alloc::string::String,
}
/// Request for the `DeleteSchema` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteSchemaRequest {
    /// Required. Name of the schema to delete.
    /// Format is `projects/{project}/schemas/{schema}`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
/// Request for the `ValidateSchema` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateSchemaRequest {
    /// Required. The name of the project in which to validate schemas.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub parent: ::prost::alloc::string::String,
    /// Required. The schema object to validate.
    #[prost(message, optional, tag = "2")]
    pub schema: ::core::option::Option<Schema>,
}
/// Response for the `ValidateSchema` method.
/// Empty for now.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateSchemaResponse {}
/// Request for the `ValidateMessage` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateMessageRequest {
    /// Required. The name of the project in which to validate schemas.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub parent: ::prost::alloc::string::String,
    /// Message to validate against the provided `schema_spec`.
    #[prost(bytes = "vec", tag = "4")]
    pub message: ::prost::alloc::vec::Vec<u8>,
    /// The encoding expected for messages
    #[prost(enumeration = "Encoding", tag = "5")]
    pub encoding: i32,
    #[prost(oneof = "validate_message_request::SchemaSpec", tags = "2, 3")]
    pub schema_spec: ::core::option::Option<validate_message_request::SchemaSpec>,
}
/// Nested message and enum types in `ValidateMessageRequest`.
pub mod validate_message_request {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum SchemaSpec {
        /// Name of the schema against which to validate.
        ///
        /// Format is `projects/{project}/schemas/{schema}`.
        #[prost(string, tag = "2")]
        Name(::prost::alloc::string::String),
        /// Ad-hoc schema against which to validate
        #[prost(message, tag = "3")]
        Schema(super::Schema),
    }
}
/// Response for the `ValidateMessage` method.
/// Empty for now.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidateMessageResponse {}
/// View of Schema object fields to be returned by GetSchema and ListSchemas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SchemaView {
    /// The default / unset value.
    /// The API will default to the BASIC view.
    Unspecified = 0,
    /// Include the name and type of the schema, but not the definition.
    Basic = 1,
    /// Include all Schema object fields.
    Full = 2,
}
impl SchemaView {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            SchemaView::Unspecified => "SCHEMA_VIEW_UNSPECIFIED",
            SchemaView::Basic => "BASIC",
            SchemaView::Full => "FULL",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SCHEMA_VIEW_UNSPECIFIED" => Some(Self::Unspecified),
            "BASIC" => Some(Self::Basic),
            "FULL" => Some(Self::Full),
            _ => None,
        }
    }
}
/// Possible encoding types for messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Encoding {
    /// Unspecified
    Unspecified = 0,
    /// JSON encoding
    Json = 1,
    /// Binary encoding, as defined by the schema type. For some schema types,
    /// binary encoding may not be available.
    Binary = 2,
}
impl Encoding {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Encoding::Unspecified => "ENCODING_UNSPECIFIED",
            Encoding::Json => "JSON",
            Encoding::Binary => "BINARY",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "ENCODING_UNSPECIFIED" => Some(Self::Unspecified),
            "JSON" => Some(Self::Json),
            "BINARY" => Some(Self::Binary),
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod schema_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Service for doing schema-related operations.
    #[derive(Debug, Clone)]
    pub struct SchemaServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SchemaServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SchemaServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SchemaServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SchemaServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Creates a schema.
        pub async fn create_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateSchemaRequest>,
        ) -> std::result::Result<tonic::Response<super::Schema>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/CreateSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "CreateSchema"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Gets a schema.
        pub async fn get_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSchemaRequest>,
        ) -> std::result::Result<tonic::Response<super::Schema>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/GetSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.SchemaService", "GetSchema"));
            self.inner.unary(req, path, codec).await
        }
        /// Lists schemas in a project.
        pub async fn list_schemas(
            &mut self,
            request: impl tonic::IntoRequest<super::ListSchemasRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListSchemasResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/ListSchemas",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "ListSchemas"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Lists all schema revisions for the named schema.
        pub async fn list_schema_revisions(
            &mut self,
            request: impl tonic::IntoRequest<super::ListSchemaRevisionsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListSchemaRevisionsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/ListSchemaRevisions",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "google.pubsub.v1.SchemaService",
                        "ListSchemaRevisions",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Commits a new schema revision to an existing schema.
        pub async fn commit_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::CommitSchemaRequest>,
        ) -> std::result::Result<tonic::Response<super::Schema>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/CommitSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "CommitSchema"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Creates a new schema revision that is a copy of the provided revision_id.
        pub async fn rollback_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::RollbackSchemaRequest>,
        ) -> std::result::Result<tonic::Response<super::Schema>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/RollbackSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "RollbackSchema"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Deletes a specific schema revision.
        pub async fn delete_schema_revision(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteSchemaRevisionRequest>,
        ) -> std::result::Result<tonic::Response<super::Schema>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/DeleteSchemaRevision",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "google.pubsub.v1.SchemaService",
                        "DeleteSchemaRevision",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Deletes a schema.
        pub async fn delete_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteSchemaRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/DeleteSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "DeleteSchema"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Validates a schema.
        pub async fn validate_schema(
            &mut self,
            request: impl tonic::IntoRequest<super::ValidateSchemaRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ValidateSchemaResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/ValidateSchema",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "ValidateSchema"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Validates a message against a schema.
        pub async fn validate_message(
            &mut self,
            request: impl tonic::IntoRequest<super::ValidateMessageRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ValidateMessageResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.SchemaService/ValidateMessage",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.SchemaService", "ValidateMessage"),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// A policy constraining the storage of messages published to the topic.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageStoragePolicy {
    /// A list of IDs of GCP regions where messages that are published to the topic
    /// may be persisted in storage. Messages published by publishers running in
    /// non-allowed GCP regions (or running outside of GCP altogether) will be
    /// routed for storage in one of the allowed regions. An empty list means that
    /// no regions are allowed, and is not a valid configuration.
    #[prost(string, repeated, tag = "1")]
    pub allowed_persistence_regions: ::prost::alloc::vec::Vec<
        ::prost::alloc::string::String,
    >,
}
/// Settings for validating messages published against a schema.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SchemaSettings {
    /// Required. The name of the schema that messages published should be
    /// validated against. Format is `projects/{project}/schemas/{schema}`. The
    /// value of this field will be `_deleted-schema_` if the schema has been
    /// deleted.
    #[prost(string, tag = "1")]
    pub schema: ::prost::alloc::string::String,
    /// The encoding of messages validated against `schema`.
    #[prost(enumeration = "Encoding", tag = "2")]
    pub encoding: i32,
    /// The minimum (inclusive) revision allowed for validating messages. If empty
    /// or not present, allow any revision to be validated against last_revision or
    /// any revision created before.
    #[prost(string, tag = "3")]
    pub first_revision_id: ::prost::alloc::string::String,
    /// The maximum (inclusive) revision allowed for validating messages. If empty
    /// or not present, allow any revision to be validated against first_revision
    /// or any revision created after.
    #[prost(string, tag = "4")]
    pub last_revision_id: ::prost::alloc::string::String,
}
/// A topic resource.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Topic {
    /// Required. The name of the topic. It must have the format
    /// `"projects/{project}/topics/{topic}"`. `{topic}` must start with a letter,
    /// and contain only letters (`\[A-Za-z\]`), numbers (`\[0-9\]`), dashes (`-`),
    /// underscores (`_`), periods (`.`), tildes (`~`), plus (`+`) or percent
    /// signs (`%`). It must be between 3 and 255 characters in length, and it
    /// must not start with `"goog"`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// See [Creating and managing labels]
    /// (<https://cloud.google.com/pubsub/docs/labels>).
    #[prost(map = "string, string", tag = "2")]
    pub labels: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// Policy constraining the set of Google Cloud Platform regions where messages
    /// published to the topic may be stored. If not present, then no constraints
    /// are in effect.
    #[prost(message, optional, tag = "3")]
    pub message_storage_policy: ::core::option::Option<MessageStoragePolicy>,
    /// The resource name of the Cloud KMS CryptoKey to be used to protect access
    /// to messages published on this topic.
    ///
    /// The expected format is `projects/*/locations/*/keyRings/*/cryptoKeys/*`.
    #[prost(string, tag = "5")]
    pub kms_key_name: ::prost::alloc::string::String,
    /// Settings for validating messages published against a schema.
    #[prost(message, optional, tag = "6")]
    pub schema_settings: ::core::option::Option<SchemaSettings>,
    /// Reserved for future use. This field is set only in responses from the
    /// server; it is ignored if it is set in any requests.
    #[prost(bool, tag = "7")]
    pub satisfies_pzs: bool,
    /// Indicates the minimum duration to retain a message after it is published to
    /// the topic. If this field is set, messages published to the topic in the
    /// last `message_retention_duration` are always available to subscribers. For
    /// instance, it allows any attached subscription to [seek to a
    /// timestamp](<https://cloud.google.com/pubsub/docs/replay-overview#seek_to_a_time>)
    /// that is up to `message_retention_duration` in the past. If this field is
    /// not set, message retention is controlled by settings on individual
    /// subscriptions. Cannot be more than 31 days or less than 10 minutes.
    #[prost(message, optional, tag = "8")]
    pub message_retention_duration: ::core::option::Option<::prost_types::Duration>,
}
/// A message that is published by publishers and consumed by subscribers. The
/// message must contain either a non-empty data field or at least one attribute.
/// Note that client libraries represent this object differently
/// depending on the language. See the corresponding [client library
/// documentation](<https://cloud.google.com/pubsub/docs/reference/libraries>) for
/// more information. See [quotas and limits]
/// (<https://cloud.google.com/pubsub/quotas>) for more information about message
/// limits.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PubsubMessage {
    /// The message data field. If this field is empty, the message must contain
    /// at least one attribute.
    #[prost(bytes = "vec", tag = "1")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// Attributes for this message. If this field is empty, the message must
    /// contain non-empty data. This can be used to filter messages on the
    /// subscription.
    #[prost(map = "string, string", tag = "2")]
    pub attributes: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// ID of this message, assigned by the server when the message is published.
    /// Guaranteed to be unique within the topic. This value may be read by a
    /// subscriber that receives a `PubsubMessage` via a `Pull` call or a push
    /// delivery. It must not be populated by the publisher in a `Publish` call.
    #[prost(string, tag = "3")]
    pub message_id: ::prost::alloc::string::String,
    /// The time at which the message was published, populated by the server when
    /// it receives the `Publish` call. It must not be populated by the
    /// publisher in a `Publish` call.
    #[prost(message, optional, tag = "4")]
    pub publish_time: ::core::option::Option<::prost_types::Timestamp>,
    /// If non-empty, identifies related messages for which publish order should be
    /// respected. If a `Subscription` has `enable_message_ordering` set to `true`,
    /// messages published with the same non-empty `ordering_key` value will be
    /// delivered to subscribers in the order in which they are received by the
    /// Pub/Sub system. All `PubsubMessage`s published in a given `PublishRequest`
    /// must specify the same `ordering_key` value.
    /// For more information, see [ordering
    /// messages](<https://cloud.google.com/pubsub/docs/ordering>).
    #[prost(string, tag = "5")]
    pub ordering_key: ::prost::alloc::string::String,
}
/// Request for the GetTopic method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTopicRequest {
    /// Required. The name of the topic to get.
    /// Format is `projects/{project}/topics/{topic}`.
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
}
/// Request for the UpdateTopic method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateTopicRequest {
    /// Required. The updated topic object.
    #[prost(message, optional, tag = "1")]
    pub topic: ::core::option::Option<Topic>,
    /// Required. Indicates which fields in the provided topic to update. Must be
    /// specified and non-empty. Note that if `update_mask` contains
    /// "message_storage_policy" but the `message_storage_policy` is not set in
    /// the `topic` provided above, then the updated value is determined by the
    /// policy configured at the project or organization level.
    #[prost(message, optional, tag = "2")]
    pub update_mask: ::core::option::Option<::prost_types::FieldMask>,
}
/// Request for the Publish method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublishRequest {
    /// Required. The messages in the request will be published on this topic.
    /// Format is `projects/{project}/topics/{topic}`.
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    /// Required. The messages to publish.
    #[prost(message, repeated, tag = "2")]
    pub messages: ::prost::alloc::vec::Vec<PubsubMessage>,
}
/// Response for the `Publish` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublishResponse {
    /// The server-assigned ID of each published message, in the same order as
    /// the messages in the request. IDs are guaranteed to be unique within
    /// the topic.
    #[prost(string, repeated, tag = "1")]
    pub message_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Request for the `ListTopics` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicsRequest {
    /// Required. The name of the project in which to list topics.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub project: ::prost::alloc::string::String,
    /// Maximum number of topics to return.
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    /// The value returned by the last `ListTopicsResponse`; indicates that this is
    /// a continuation of a prior `ListTopics` call, and that the system should
    /// return the next page of data.
    #[prost(string, tag = "3")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListTopics` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicsResponse {
    /// The resulting topics.
    #[prost(message, repeated, tag = "1")]
    pub topics: ::prost::alloc::vec::Vec<Topic>,
    /// If not empty, indicates that there may be more topics that match the
    /// request; this value should be passed in a new `ListTopicsRequest`.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the `ListTopicSubscriptions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicSubscriptionsRequest {
    /// Required. The name of the topic that subscriptions are attached to.
    /// Format is `projects/{project}/topics/{topic}`.
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    /// Maximum number of subscription names to return.
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    /// The value returned by the last `ListTopicSubscriptionsResponse`; indicates
    /// that this is a continuation of a prior `ListTopicSubscriptions` call, and
    /// that the system should return the next page of data.
    #[prost(string, tag = "3")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListTopicSubscriptions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicSubscriptionsResponse {
    /// The names of subscriptions attached to the topic specified in the request.
    #[prost(string, repeated, tag = "1")]
    pub subscriptions: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// If not empty, indicates that there may be more subscriptions that match
    /// the request; this value should be passed in a new
    /// `ListTopicSubscriptionsRequest` to get more subscriptions.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the `ListTopicSnapshots` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicSnapshotsRequest {
    /// Required. The name of the topic that snapshots are attached to.
    /// Format is `projects/{project}/topics/{topic}`.
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    /// Maximum number of snapshot names to return.
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    /// The value returned by the last `ListTopicSnapshotsResponse`; indicates
    /// that this is a continuation of a prior `ListTopicSnapshots` call, and
    /// that the system should return the next page of data.
    #[prost(string, tag = "3")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListTopicSnapshots` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTopicSnapshotsResponse {
    /// The names of the snapshots that match the request.
    #[prost(string, repeated, tag = "1")]
    pub snapshots: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// If not empty, indicates that there may be more snapshots that match
    /// the request; this value should be passed in a new
    /// `ListTopicSnapshotsRequest` to get more snapshots.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the `DeleteTopic` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTopicRequest {
    /// Required. Name of the topic to delete.
    /// Format is `projects/{project}/topics/{topic}`.
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
}
/// Request for the DetachSubscription method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DetachSubscriptionRequest {
    /// Required. The subscription to detach.
    /// Format is `projects/{project}/subscriptions/{subscription}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
}
/// Response for the DetachSubscription method.
/// Reserved for future use.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DetachSubscriptionResponse {}
/// A subscription resource. If none of `push_config`, `bigquery_config`, or
/// `cloud_storage_config` is set, then the subscriber will pull and ack messages
/// using API methods. At most one of these fields may be set.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Subscription {
    /// Required. The name of the subscription. It must have the format
    /// `"projects/{project}/subscriptions/{subscription}"`. `{subscription}` must
    /// start with a letter, and contain only letters (`\[A-Za-z\]`), numbers
    /// (`\[0-9\]`), dashes (`-`), underscores (`_`), periods (`.`), tildes (`~`),
    /// plus (`+`) or percent signs (`%`). It must be between 3 and 255 characters
    /// in length, and it must not start with `"goog"`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Required. The name of the topic from which this subscription is receiving
    /// messages. Format is `projects/{project}/topics/{topic}`. The value of this
    /// field will be `_deleted-topic_` if the topic has been deleted.
    #[prost(string, tag = "2")]
    pub topic: ::prost::alloc::string::String,
    /// If push delivery is used with this subscription, this field is
    /// used to configure it.
    #[prost(message, optional, tag = "4")]
    pub push_config: ::core::option::Option<PushConfig>,
    /// If delivery to BigQuery is used with this subscription, this field is
    /// used to configure it.
    #[prost(message, optional, tag = "18")]
    pub bigquery_config: ::core::option::Option<BigQueryConfig>,
    /// If delivery to Google Cloud Storage is used with this subscription, this
    /// field is used to configure it.
    #[prost(message, optional, tag = "22")]
    pub cloud_storage_config: ::core::option::Option<CloudStorageConfig>,
    /// The approximate amount of time (on a best-effort basis) Pub/Sub waits for
    /// the subscriber to acknowledge receipt before resending the message. In the
    /// interval after the message is delivered and before it is acknowledged, it
    /// is considered to be _outstanding_. During that time period, the
    /// message will not be redelivered (on a best-effort basis).
    ///
    /// For pull subscriptions, this value is used as the initial value for the ack
    /// deadline. To override this value for a given message, call
    /// `ModifyAckDeadline` with the corresponding `ack_id` if using
    /// non-streaming pull or send the `ack_id` in a
    /// `StreamingModifyAckDeadlineRequest` if using streaming pull.
    /// The minimum custom deadline you can specify is 10 seconds.
    /// The maximum custom deadline you can specify is 600 seconds (10 minutes).
    /// If this parameter is 0, a default value of 10 seconds is used.
    ///
    /// For push delivery, this value is also used to set the request timeout for
    /// the call to the push endpoint.
    ///
    /// If the subscriber never acknowledges the message, the Pub/Sub
    /// system will eventually redeliver the message.
    #[prost(int32, tag = "5")]
    pub ack_deadline_seconds: i32,
    /// Indicates whether to retain acknowledged messages. If true, then
    /// messages are not expunged from the subscription's backlog, even if they are
    /// acknowledged, until they fall out of the `message_retention_duration`
    /// window. This must be true if you would like to [`Seek` to a timestamp]
    /// (<https://cloud.google.com/pubsub/docs/replay-overview#seek_to_a_time>) in
    /// the past to replay previously-acknowledged messages.
    #[prost(bool, tag = "7")]
    pub retain_acked_messages: bool,
    /// How long to retain unacknowledged messages in the subscription's backlog,
    /// from the moment a message is published.
    /// If `retain_acked_messages` is true, then this also configures the retention
    /// of acknowledged messages, and thus configures how far back in time a `Seek`
    /// can be done. Defaults to 7 days. Cannot be more than 7 days or less than 10
    /// minutes.
    #[prost(message, optional, tag = "8")]
    pub message_retention_duration: ::core::option::Option<::prost_types::Duration>,
    /// See [Creating and managing
    /// labels](<https://cloud.google.com/pubsub/docs/labels>).
    #[prost(map = "string, string", tag = "9")]
    pub labels: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// If true, messages published with the same `ordering_key` in `PubsubMessage`
    /// will be delivered to the subscribers in the order in which they
    /// are received by the Pub/Sub system. Otherwise, they may be delivered in
    /// any order.
    #[prost(bool, tag = "10")]
    pub enable_message_ordering: bool,
    /// A policy that specifies the conditions for this subscription's expiration.
    /// A subscription is considered active as long as any connected subscriber is
    /// successfully consuming messages from the subscription or is issuing
    /// operations on the subscription. If `expiration_policy` is not set, a
    /// *default policy* with `ttl` of 31 days will be used. The minimum allowed
    /// value for `expiration_policy.ttl` is 1 day. If `expiration_policy` is set,
    /// but `expiration_policy.ttl` is not set, the subscription never expires.
    #[prost(message, optional, tag = "11")]
    pub expiration_policy: ::core::option::Option<ExpirationPolicy>,
    /// An expression written in the Pub/Sub [filter
    /// language](<https://cloud.google.com/pubsub/docs/filtering>). If non-empty,
    /// then only `PubsubMessage`s whose `attributes` field matches the filter are
    /// delivered on this subscription. If empty, then no messages are filtered
    /// out.
    #[prost(string, tag = "12")]
    pub filter: ::prost::alloc::string::String,
    /// A policy that specifies the conditions for dead lettering messages in
    /// this subscription. If dead_letter_policy is not set, dead lettering
    /// is disabled.
    ///
    /// The Cloud Pub/Sub service account associated with this subscriptions's
    /// parent project (i.e.,
    /// service-{project_number}@gcp-sa-pubsub.iam.gserviceaccount.com) must have
    /// permission to Acknowledge() messages on this subscription.
    #[prost(message, optional, tag = "13")]
    pub dead_letter_policy: ::core::option::Option<DeadLetterPolicy>,
    /// A policy that specifies how Pub/Sub retries message delivery for this
    /// subscription.
    ///
    /// If not set, the default retry policy is applied. This generally implies
    /// that messages will be retried as soon as possible for healthy subscribers.
    /// RetryPolicy will be triggered on NACKs or acknowledgement deadline
    /// exceeded events for a given message.
    #[prost(message, optional, tag = "14")]
    pub retry_policy: ::core::option::Option<RetryPolicy>,
    /// Indicates whether the subscription is detached from its topic. Detached
    /// subscriptions don't receive messages from their topic and don't retain any
    /// backlog. `Pull` and `StreamingPull` requests will return
    /// FAILED_PRECONDITION. If the subscription is a push subscription, pushes to
    /// the endpoint will not be made.
    #[prost(bool, tag = "15")]
    pub detached: bool,
    /// If true, Pub/Sub provides the following guarantees for the delivery of
    /// a message with a given value of `message_id` on this subscription:
    ///
    /// * The message sent to a subscriber is guaranteed not to be resent
    /// before the message's acknowledgement deadline expires.
    /// * An acknowledged message will not be resent to a subscriber.
    ///
    /// Note that subscribers may still receive multiple copies of a message
    /// when `enable_exactly_once_delivery` is true if the message was published
    /// multiple times by a publisher client. These copies are  considered distinct
    /// by Pub/Sub and have distinct `message_id` values.
    #[prost(bool, tag = "16")]
    pub enable_exactly_once_delivery: bool,
    /// Output only. Indicates the minimum duration for which a message is retained
    /// after it is published to the subscription's topic. If this field is set,
    /// messages published to the subscription's topic in the last
    /// `topic_message_retention_duration` are always available to subscribers. See
    /// the `message_retention_duration` field in `Topic`. This field is set only
    /// in responses from the server; it is ignored if it is set in any requests.
    #[prost(message, optional, tag = "17")]
    pub topic_message_retention_duration: ::core::option::Option<
        ::prost_types::Duration,
    >,
    /// Output only. An output-only field indicating whether or not the
    /// subscription can receive messages.
    #[prost(enumeration = "subscription::State", tag = "19")]
    pub state: i32,
}
/// Nested message and enum types in `Subscription`.
pub mod subscription {
    /// Possible states for a subscription.
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum State {
        /// Default value. This value is unused.
        Unspecified = 0,
        /// The subscription can actively receive messages
        Active = 1,
        /// The subscription cannot receive messages because of an error with the
        /// resource to which it pushes messages. See the more detailed error state
        /// in the corresponding configuration.
        ResourceError = 2,
    }
    impl State {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                State::Unspecified => "STATE_UNSPECIFIED",
                State::Active => "ACTIVE",
                State::ResourceError => "RESOURCE_ERROR",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "STATE_UNSPECIFIED" => Some(Self::Unspecified),
                "ACTIVE" => Some(Self::Active),
                "RESOURCE_ERROR" => Some(Self::ResourceError),
                _ => None,
            }
        }
    }
}
/// A policy that specifies how Cloud Pub/Sub retries message delivery.
///
/// Retry delay will be exponential based on provided minimum and maximum
/// backoffs. <https://en.wikipedia.org/wiki/Exponential_backoff.>
///
/// RetryPolicy will be triggered on NACKs or acknowledgement deadline exceeded
/// events for a given message.
///
/// Retry Policy is implemented on a best effort basis. At times, the delay
/// between consecutive deliveries may not match the configuration. That is,
/// delay can be more or less than configured backoff.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RetryPolicy {
    /// The minimum delay between consecutive deliveries of a given message.
    /// Value should be between 0 and 600 seconds. Defaults to 10 seconds.
    #[prost(message, optional, tag = "1")]
    pub minimum_backoff: ::core::option::Option<::prost_types::Duration>,
    /// The maximum delay between consecutive deliveries of a given message.
    /// Value should be between 0 and 600 seconds. Defaults to 600 seconds.
    #[prost(message, optional, tag = "2")]
    pub maximum_backoff: ::core::option::Option<::prost_types::Duration>,
}
/// Dead lettering is done on a best effort basis. The same message might be
/// dead lettered multiple times.
///
/// If validation on any of the fields fails at subscription creation/updation,
/// the create/update subscription request will fail.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeadLetterPolicy {
    /// The name of the topic to which dead letter messages should be published.
    /// Format is `projects/{project}/topics/{topic}`.The Cloud Pub/Sub service
    /// account associated with the enclosing subscription's parent project (i.e.,
    /// service-{project_number}@gcp-sa-pubsub.iam.gserviceaccount.com) must have
    /// permission to Publish() to this topic.
    ///
    /// The operation will fail if the topic does not exist.
    /// Users should ensure that there is a subscription attached to this topic
    /// since messages published to a topic with no subscriptions are lost.
    #[prost(string, tag = "1")]
    pub dead_letter_topic: ::prost::alloc::string::String,
    /// The maximum number of delivery attempts for any message. The value must be
    /// between 5 and 100.
    ///
    /// The number of delivery attempts is defined as 1 + (the sum of number of
    /// NACKs and number of times the acknowledgement deadline has been exceeded
    /// for the message).
    ///
    /// A NACK is any call to ModifyAckDeadline with a 0 deadline. Note that
    /// client libraries may automatically extend ack_deadlines.
    ///
    /// This field will be honored on a best effort basis.
    ///
    /// If this parameter is 0, a default value of 5 is used.
    #[prost(int32, tag = "2")]
    pub max_delivery_attempts: i32,
}
/// A policy that specifies the conditions for resource expiration (i.e.,
/// automatic resource deletion).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExpirationPolicy {
    /// Specifies the "time-to-live" duration for an associated resource. The
    /// resource expires if it is not active for a period of `ttl`. The definition
    /// of "activity" depends on the type of the associated resource. The minimum
    /// and maximum allowed values for `ttl` depend on the type of the associated
    /// resource, as well. If `ttl` is not set, the associated resource never
    /// expires.
    #[prost(message, optional, tag = "1")]
    pub ttl: ::core::option::Option<::prost_types::Duration>,
}
/// Configuration for a push delivery endpoint.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushConfig {
    /// A URL locating the endpoint to which messages should be pushed.
    /// For example, a Webhook endpoint might use `<https://example.com/push`.>
    #[prost(string, tag = "1")]
    pub push_endpoint: ::prost::alloc::string::String,
    /// Endpoint configuration attributes that can be used to control different
    /// aspects of the message delivery.
    ///
    /// The only currently supported attribute is `x-goog-version`, which you can
    /// use to change the format of the pushed message. This attribute
    /// indicates the version of the data expected by the endpoint. This
    /// controls the shape of the pushed message (i.e., its fields and metadata).
    ///
    /// If not present during the `CreateSubscription` call, it will default to
    /// the version of the Pub/Sub API used to make such call. If not present in a
    /// `ModifyPushConfig` call, its value will not be changed. `GetSubscription`
    /// calls will always return a valid version, even if the subscription was
    /// created without this attribute.
    ///
    /// The only supported values for the `x-goog-version` attribute are:
    ///
    /// * `v1beta1`: uses the push format defined in the v1beta1 Pub/Sub API.
    /// * `v1` or `v1beta2`: uses the push format defined in the v1 Pub/Sub API.
    ///
    /// For example:
    /// `attributes { "x-goog-version": "v1" }`
    #[prost(map = "string, string", tag = "2")]
    pub attributes: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// An authentication method used by push endpoints to verify the source of
    /// push requests. This can be used with push endpoints that are private by
    /// default to allow requests only from the Cloud Pub/Sub system, for example.
    /// This field is optional and should be set only by users interested in
    /// authenticated push.
    #[prost(oneof = "push_config::AuthenticationMethod", tags = "3")]
    pub authentication_method: ::core::option::Option<push_config::AuthenticationMethod>,
    /// The format of the delivered message to the push endpoint is defined by
    /// the chosen wrapper. When unset, `PubsubWrapper` is used.
    #[prost(oneof = "push_config::Wrapper", tags = "4, 5")]
    pub wrapper: ::core::option::Option<push_config::Wrapper>,
}
/// Nested message and enum types in `PushConfig`.
pub mod push_config {
    /// Contains information needed for generating an
    /// [OpenID Connect
    /// token](<https://developers.google.com/identity/protocols/OpenIDConnect>).
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct OidcToken {
        /// [Service account
        /// email](<https://cloud.google.com/iam/docs/service-accounts>)
        /// used for generating the OIDC token. For more information
        /// on setting up authentication, see
        /// [Push subscriptions](<https://cloud.google.com/pubsub/docs/push>).
        #[prost(string, tag = "1")]
        pub service_account_email: ::prost::alloc::string::String,
        /// Audience to be used when generating OIDC token. The audience claim
        /// identifies the recipients that the JWT is intended for. The audience
        /// value is a single case-sensitive string. Having multiple values (array)
        /// for the audience field is not supported. More info about the OIDC JWT
        /// token audience here: <https://tools.ietf.org/html/rfc7519#section-4.1.3>
        /// Note: if not specified, the Push endpoint URL will be used.
        #[prost(string, tag = "2")]
        pub audience: ::prost::alloc::string::String,
    }
    /// The payload to the push endpoint is in the form of the JSON representation
    /// of a PubsubMessage
    /// (<https://cloud.google.com/pubsub/docs/reference/rpc/google.pubsub.v1#pubsubmessage>).
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PubsubWrapper {}
    /// Sets the `data` field as the HTTP body for delivery.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct NoWrapper {
        /// When true, writes the Pub/Sub message metadata to
        /// `x-goog-pubsub-<KEY>:<VAL>` headers of the HTTP request. Writes the
        /// Pub/Sub message attributes to `<KEY>:<VAL>` headers of the HTTP request.
        #[prost(bool, tag = "1")]
        pub write_metadata: bool,
    }
    /// An authentication method used by push endpoints to verify the source of
    /// push requests. This can be used with push endpoints that are private by
    /// default to allow requests only from the Cloud Pub/Sub system, for example.
    /// This field is optional and should be set only by users interested in
    /// authenticated push.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum AuthenticationMethod {
        /// If specified, Pub/Sub will generate and attach an OIDC JWT token as an
        /// `Authorization` header in the HTTP request for every pushed message.
        #[prost(message, tag = "3")]
        OidcToken(OidcToken),
    }
    /// The format of the delivered message to the push endpoint is defined by
    /// the chosen wrapper. When unset, `PubsubWrapper` is used.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Wrapper {
        /// When set, the payload to the push endpoint is in the form of the JSON
        /// representation of a PubsubMessage
        /// (<https://cloud.google.com/pubsub/docs/reference/rpc/google.pubsub.v1#pubsubmessage>).
        #[prost(message, tag = "4")]
        PubsubWrapper(PubsubWrapper),
        /// When set, the payload to the push endpoint is not wrapped.
        #[prost(message, tag = "5")]
        NoWrapper(NoWrapper),
    }
}
/// Configuration for a BigQuery subscription.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BigQueryConfig {
    /// The name of the table to which to write data, of the form
    /// {projectId}.{datasetId}.{tableId}
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    /// When true, use the topic's schema as the columns to write to in BigQuery,
    /// if it exists.
    #[prost(bool, tag = "2")]
    pub use_topic_schema: bool,
    /// When true, write the subscription name, message_id, publish_time,
    /// attributes, and ordering_key to additional columns in the table. The
    /// subscription name, message_id, and publish_time fields are put in their own
    /// columns while all other message properties (other than data) are written to
    /// a JSON object in the attributes column.
    #[prost(bool, tag = "3")]
    pub write_metadata: bool,
    /// When true and use_topic_schema is true, any fields that are a part of the
    /// topic schema that are not part of the BigQuery table schema are dropped
    /// when writing to BigQuery. Otherwise, the schemas must be kept in sync and
    /// any messages with extra fields are not written and remain in the
    /// subscription's backlog.
    #[prost(bool, tag = "4")]
    pub drop_unknown_fields: bool,
    /// Output only. An output-only field that indicates whether or not the
    /// subscription can receive messages.
    #[prost(enumeration = "big_query_config::State", tag = "5")]
    pub state: i32,
}
/// Nested message and enum types in `BigQueryConfig`.
pub mod big_query_config {
    /// Possible states for a BigQuery subscription.
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum State {
        /// Default value. This value is unused.
        Unspecified = 0,
        /// The subscription can actively send messages to BigQuery
        Active = 1,
        /// Cannot write to the BigQuery table because of permission denied errors.
        /// This can happen if
        /// - Pub/Sub SA has not been granted the [appropriate BigQuery IAM
        /// permissions](<https://cloud.google.com/pubsub/docs/create-subscription#assign_bigquery_service_account>)
        /// - bigquery.googleapis.com API is not enabled for the project
        /// (\[instructions\](<https://cloud.google.com/service-usage/docs/enable-disable>))
        PermissionDenied = 2,
        /// Cannot write to the BigQuery table because it does not exist.
        NotFound = 3,
        /// Cannot write to the BigQuery table due to a schema mismatch.
        SchemaMismatch = 4,
    }
    impl State {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                State::Unspecified => "STATE_UNSPECIFIED",
                State::Active => "ACTIVE",
                State::PermissionDenied => "PERMISSION_DENIED",
                State::NotFound => "NOT_FOUND",
                State::SchemaMismatch => "SCHEMA_MISMATCH",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "STATE_UNSPECIFIED" => Some(Self::Unspecified),
                "ACTIVE" => Some(Self::Active),
                "PERMISSION_DENIED" => Some(Self::PermissionDenied),
                "NOT_FOUND" => Some(Self::NotFound),
                "SCHEMA_MISMATCH" => Some(Self::SchemaMismatch),
                _ => None,
            }
        }
    }
}
/// Configuration for a Cloud Storage subscription.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CloudStorageConfig {
    /// Required. User-provided name for the Cloud Storage bucket.
    /// The bucket must be created by the user. The bucket name must be without
    /// any prefix like "gs://". See the [bucket naming
    /// requirements] (<https://cloud.google.com/storage/docs/buckets#naming>).
    #[prost(string, tag = "1")]
    pub bucket: ::prost::alloc::string::String,
    /// User-provided prefix for Cloud Storage filename. See the [object naming
    /// requirements](<https://cloud.google.com/storage/docs/objects#naming>).
    #[prost(string, tag = "2")]
    pub filename_prefix: ::prost::alloc::string::String,
    /// User-provided suffix for Cloud Storage filename. See the [object naming
    /// requirements](<https://cloud.google.com/storage/docs/objects#naming>). Must
    /// not end in "/".
    #[prost(string, tag = "3")]
    pub filename_suffix: ::prost::alloc::string::String,
    /// The maximum duration that can elapse before a new Cloud Storage file is
    /// created. Min 1 minute, max 10 minutes, default 5 minutes. May not exceed
    /// the subscription's acknowledgement deadline.
    #[prost(message, optional, tag = "6")]
    pub max_duration: ::core::option::Option<::prost_types::Duration>,
    /// The maximum bytes that can be written to a Cloud Storage file before a new
    /// file is created. Min 1 KB, max 10 GiB. The max_bytes limit may be exceeded
    /// in cases where messages are larger than the limit.
    #[prost(int64, tag = "7")]
    pub max_bytes: i64,
    /// Output only. An output-only field that indicates whether or not the
    /// subscription can receive messages.
    #[prost(enumeration = "cloud_storage_config::State", tag = "9")]
    pub state: i32,
    /// Defaults to text format.
    #[prost(oneof = "cloud_storage_config::OutputFormat", tags = "4, 5")]
    pub output_format: ::core::option::Option<cloud_storage_config::OutputFormat>,
}
/// Nested message and enum types in `CloudStorageConfig`.
pub mod cloud_storage_config {
    /// Configuration for writing message data in text format.
    /// Message payloads will be written to files as raw text, separated by a
    /// newline.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct TextConfig {}
    /// Configuration for writing message data in Avro format.
    /// Message payloads and metadata will be written to files as an Avro binary.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AvroConfig {
        /// When true, write the subscription name, message_id, publish_time,
        /// attributes, and ordering_key as additional fields in the output.
        #[prost(bool, tag = "1")]
        pub write_metadata: bool,
    }
    /// Possible states for a Cloud Storage subscription.
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum State {
        /// Default value. This value is unused.
        Unspecified = 0,
        /// The subscription can actively send messages to Cloud Storage.
        Active = 1,
        /// Cannot write to the Cloud Storage bucket because of permission denied
        /// errors.
        PermissionDenied = 2,
        /// Cannot write to the Cloud Storage bucket because it does not exist.
        NotFound = 3,
    }
    impl State {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                State::Unspecified => "STATE_UNSPECIFIED",
                State::Active => "ACTIVE",
                State::PermissionDenied => "PERMISSION_DENIED",
                State::NotFound => "NOT_FOUND",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "STATE_UNSPECIFIED" => Some(Self::Unspecified),
                "ACTIVE" => Some(Self::Active),
                "PERMISSION_DENIED" => Some(Self::PermissionDenied),
                "NOT_FOUND" => Some(Self::NotFound),
                _ => None,
            }
        }
    }
    /// Defaults to text format.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum OutputFormat {
        /// If set, message data will be written to Cloud Storage in text format.
        #[prost(message, tag = "4")]
        TextConfig(TextConfig),
        /// If set, message data will be written to Cloud Storage in Avro format.
        #[prost(message, tag = "5")]
        AvroConfig(AvroConfig),
    }
}
/// A message and its corresponding acknowledgment ID.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReceivedMessage {
    /// This ID can be used to acknowledge the received message.
    #[prost(string, tag = "1")]
    pub ack_id: ::prost::alloc::string::String,
    /// The message.
    #[prost(message, optional, tag = "2")]
    pub message: ::core::option::Option<PubsubMessage>,
    /// The approximate number of times that Cloud Pub/Sub has attempted to deliver
    /// the associated message to a subscriber.
    ///
    /// More precisely, this is 1 + (number of NACKs) +
    /// (number of ack_deadline exceeds) for this message.
    ///
    /// A NACK is any call to ModifyAckDeadline with a 0 deadline. An ack_deadline
    /// exceeds event is whenever a message is not acknowledged within
    /// ack_deadline. Note that ack_deadline is initially
    /// Subscription.ackDeadlineSeconds, but may get extended automatically by
    /// the client library.
    ///
    /// Upon the first delivery of a given message, `delivery_attempt` will have a
    /// value of 1. The value is calculated at best effort and is approximate.
    ///
    /// If a DeadLetterPolicy is not set on the subscription, this will be 0.
    #[prost(int32, tag = "3")]
    pub delivery_attempt: i32,
}
/// Request for the GetSubscription method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSubscriptionRequest {
    /// Required. The name of the subscription to get.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
}
/// Request for the UpdateSubscription method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateSubscriptionRequest {
    /// Required. The updated subscription object.
    #[prost(message, optional, tag = "1")]
    pub subscription: ::core::option::Option<Subscription>,
    /// Required. Indicates which fields in the provided subscription to update.
    /// Must be specified and non-empty.
    #[prost(message, optional, tag = "2")]
    pub update_mask: ::core::option::Option<::prost_types::FieldMask>,
}
/// Request for the `ListSubscriptions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSubscriptionsRequest {
    /// Required. The name of the project in which to list subscriptions.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub project: ::prost::alloc::string::String,
    /// Maximum number of subscriptions to return.
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    /// The value returned by the last `ListSubscriptionsResponse`; indicates that
    /// this is a continuation of a prior `ListSubscriptions` call, and that the
    /// system should return the next page of data.
    #[prost(string, tag = "3")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListSubscriptions` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSubscriptionsResponse {
    /// The subscriptions that match the request.
    #[prost(message, repeated, tag = "1")]
    pub subscriptions: ::prost::alloc::vec::Vec<Subscription>,
    /// If not empty, indicates that there may be more subscriptions that match
    /// the request; this value should be passed in a new
    /// `ListSubscriptionsRequest` to get more subscriptions.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the DeleteSubscription method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteSubscriptionRequest {
    /// Required. The subscription to delete.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
}
/// Request for the ModifyPushConfig method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModifyPushConfigRequest {
    /// Required. The name of the subscription.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    /// Required. The push configuration for future deliveries.
    ///
    /// An empty `pushConfig` indicates that the Pub/Sub system should
    /// stop pushing messages from the given subscription and allow
    /// messages to be pulled and acknowledged - effectively pausing
    /// the subscription if `Pull` or `StreamingPull` is not called.
    #[prost(message, optional, tag = "2")]
    pub push_config: ::core::option::Option<PushConfig>,
}
/// Request for the `Pull` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PullRequest {
    /// Required. The subscription from which messages should be pulled.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    /// Optional. If this field set to true, the system will respond immediately
    /// even if it there are no messages available to return in the `Pull`
    /// response. Otherwise, the system may wait (for a bounded amount of time)
    /// until at least one message is available, rather than returning no messages.
    /// Warning: setting this field to `true` is discouraged because it adversely
    /// impacts the performance of `Pull` operations. We recommend that users do
    /// not set this field.
    #[deprecated]
    #[prost(bool, tag = "2")]
    pub return_immediately: bool,
    /// Required. The maximum number of messages to return for this request. Must
    /// be a positive integer. The Pub/Sub system may return fewer than the number
    /// specified.
    #[prost(int32, tag = "3")]
    pub max_messages: i32,
}
/// Response for the `Pull` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PullResponse {
    /// Received Pub/Sub messages. The list will be empty if there are no more
    /// messages available in the backlog, or if no messages could be returned
    /// before the request timeout. For JSON, the response can be entirely
    /// empty. The Pub/Sub system may return fewer than the `maxMessages` requested
    /// even if there are more messages available in the backlog.
    #[prost(message, repeated, tag = "1")]
    pub received_messages: ::prost::alloc::vec::Vec<ReceivedMessage>,
}
/// Request for the ModifyAckDeadline method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModifyAckDeadlineRequest {
    /// Required. The name of the subscription.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    /// Required. List of acknowledgment IDs.
    #[prost(string, repeated, tag = "4")]
    pub ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Required. The new ack deadline with respect to the time this request was
    /// sent to the Pub/Sub system. For example, if the value is 10, the new ack
    /// deadline will expire 10 seconds after the `ModifyAckDeadline` call was
    /// made. Specifying zero might immediately make the message available for
    /// delivery to another subscriber client. This typically results in an
    /// increase in the rate of message redeliveries (that is, duplicates).
    /// The minimum deadline you can specify is 0 seconds.
    /// The maximum deadline you can specify is 600 seconds (10 minutes).
    #[prost(int32, tag = "3")]
    pub ack_deadline_seconds: i32,
}
/// Request for the Acknowledge method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AcknowledgeRequest {
    /// Required. The subscription whose message is being acknowledged.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    /// Required. The acknowledgment ID for the messages being acknowledged that
    /// was returned by the Pub/Sub system in the `Pull` response. Must not be
    /// empty.
    #[prost(string, repeated, tag = "2")]
    pub ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Request for the `StreamingPull` streaming RPC method. This request is used to
/// establish the initial stream as well as to stream acknowledgements and ack
/// deadline modifications from the client to the server.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreamingPullRequest {
    /// Required. The subscription for which to initialize the new stream. This
    /// must be provided in the first request on the stream, and must not be set in
    /// subsequent requests from client to server.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    /// List of acknowledgement IDs for acknowledging previously received messages
    /// (received on this stream or a different stream). If an ack ID has expired,
    /// the corresponding message may be redelivered later. Acknowledging a message
    /// more than once will not result in an error. If the acknowledgement ID is
    /// malformed, the stream will be aborted with status `INVALID_ARGUMENT`.
    #[prost(string, repeated, tag = "2")]
    pub ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// The list of new ack deadlines for the IDs listed in
    /// `modify_deadline_ack_ids`. The size of this list must be the same as the
    /// size of `modify_deadline_ack_ids`. If it differs the stream will be aborted
    /// with `INVALID_ARGUMENT`. Each element in this list is applied to the
    /// element in the same position in `modify_deadline_ack_ids`. The new ack
    /// deadline is with respect to the time this request was sent to the Pub/Sub
    /// system. Must be >= 0. For example, if the value is 10, the new ack deadline
    /// will expire 10 seconds after this request is received. If the value is 0,
    /// the message is immediately made available for another streaming or
    /// non-streaming pull request. If the value is < 0 (an error), the stream will
    /// be aborted with status `INVALID_ARGUMENT`.
    #[prost(int32, repeated, tag = "3")]
    pub modify_deadline_seconds: ::prost::alloc::vec::Vec<i32>,
    /// List of acknowledgement IDs whose deadline will be modified based on the
    /// corresponding element in `modify_deadline_seconds`. This field can be used
    /// to indicate that more time is needed to process a message by the
    /// subscriber, or to make the message available for redelivery if the
    /// processing was interrupted.
    #[prost(string, repeated, tag = "4")]
    pub modify_deadline_ack_ids: ::prost::alloc::vec::Vec<
        ::prost::alloc::string::String,
    >,
    /// Required. The ack deadline to use for the stream. This must be provided in
    /// the first request on the stream, but it can also be updated on subsequent
    /// requests from client to server. The minimum deadline you can specify is 10
    /// seconds. The maximum deadline you can specify is 600 seconds (10 minutes).
    #[prost(int32, tag = "5")]
    pub stream_ack_deadline_seconds: i32,
    /// A unique identifier that is used to distinguish client instances from each
    /// other. Only needs to be provided on the initial request. When a stream
    /// disconnects and reconnects for the same stream, the client_id should be set
    /// to the same value so that state associated with the old stream can be
    /// transferred to the new stream. The same client_id should not be used for
    /// different client instances.
    #[prost(string, tag = "6")]
    pub client_id: ::prost::alloc::string::String,
    /// Flow control settings for the maximum number of outstanding messages. When
    /// there are `max_outstanding_messages` or more currently sent to the
    /// streaming pull client that have not yet been acked or nacked, the server
    /// stops sending more messages. The sending of messages resumes once the
    /// number of outstanding messages is less than this value. If the value is
    /// <= 0, there is no limit to the number of outstanding messages. This
    /// property can only be set on the initial StreamingPullRequest. If it is set
    /// on a subsequent request, the stream will be aborted with status
    /// `INVALID_ARGUMENT`.
    #[prost(int64, tag = "7")]
    pub max_outstanding_messages: i64,
    /// Flow control settings for the maximum number of outstanding bytes. When
    /// there are `max_outstanding_bytes` or more worth of messages currently sent
    /// to the streaming pull client that have not yet been acked or nacked, the
    /// server will stop sending more messages. The sending of messages resumes
    /// once the number of outstanding bytes is less than this value. If the value
    /// is <= 0, there is no limit to the number of outstanding bytes. This
    /// property can only be set on the initial StreamingPullRequest. If it is set
    /// on a subsequent request, the stream will be aborted with status
    /// `INVALID_ARGUMENT`.
    #[prost(int64, tag = "8")]
    pub max_outstanding_bytes: i64,
}
/// Response for the `StreamingPull` method. This response is used to stream
/// messages from the server to the client.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreamingPullResponse {
    /// Received Pub/Sub messages. This will not be empty.
    #[prost(message, repeated, tag = "1")]
    pub received_messages: ::prost::alloc::vec::Vec<ReceivedMessage>,
    /// This field will only be set if `enable_exactly_once_delivery` is set to
    /// `true`.
    #[prost(message, optional, tag = "5")]
    pub acknowledge_confirmation: ::core::option::Option<
        streaming_pull_response::AcknowledgeConfirmation,
    >,
    /// This field will only be set if `enable_exactly_once_delivery` is set to
    /// `true`.
    #[prost(message, optional, tag = "3")]
    pub modify_ack_deadline_confirmation: ::core::option::Option<
        streaming_pull_response::ModifyAckDeadlineConfirmation,
    >,
    /// Properties associated with this subscription.
    #[prost(message, optional, tag = "4")]
    pub subscription_properties: ::core::option::Option<
        streaming_pull_response::SubscriptionProperties,
    >,
}
/// Nested message and enum types in `StreamingPullResponse`.
pub mod streaming_pull_response {
    /// Acknowledgement IDs sent in one or more previous requests to acknowledge a
    /// previously received message.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AcknowledgeConfirmation {
        /// Successfully processed acknowledgement IDs.
        #[prost(string, repeated, tag = "1")]
        pub ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
        /// List of acknowledgement IDs that were malformed or whose acknowledgement
        /// deadline has expired.
        #[prost(string, repeated, tag = "2")]
        pub invalid_ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
        /// List of acknowledgement IDs that were out of order.
        #[prost(string, repeated, tag = "3")]
        pub unordered_ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
        /// List of acknowledgement IDs that failed processing with temporary issues.
        #[prost(string, repeated, tag = "4")]
        pub temporary_failed_ack_ids: ::prost::alloc::vec::Vec<
            ::prost::alloc::string::String,
        >,
    }
    /// Acknowledgement IDs sent in one or more previous requests to modify the
    /// deadline for a specific message.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ModifyAckDeadlineConfirmation {
        /// Successfully processed acknowledgement IDs.
        #[prost(string, repeated, tag = "1")]
        pub ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
        /// List of acknowledgement IDs that were malformed or whose acknowledgement
        /// deadline has expired.
        #[prost(string, repeated, tag = "2")]
        pub invalid_ack_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
        /// List of acknowledgement IDs that failed processing with temporary issues.
        #[prost(string, repeated, tag = "3")]
        pub temporary_failed_ack_ids: ::prost::alloc::vec::Vec<
            ::prost::alloc::string::String,
        >,
    }
    /// Subscription properties sent as part of the response.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubscriptionProperties {
        /// True iff exactly once delivery is enabled for this subscription.
        #[prost(bool, tag = "1")]
        pub exactly_once_delivery_enabled: bool,
        /// True iff message ordering is enabled for this subscription.
        #[prost(bool, tag = "2")]
        pub message_ordering_enabled: bool,
    }
}
/// Request for the `CreateSnapshot` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateSnapshotRequest {
    /// Required. User-provided name for this snapshot. If the name is not provided
    /// in the request, the server will assign a random name for this snapshot on
    /// the same project as the subscription. Note that for REST API requests, you
    /// must specify a name.  See the [resource name
    /// rules](<https://cloud.google.com/pubsub/docs/admin#resource_names>). Format
    /// is `projects/{project}/snapshots/{snap}`.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// Required. The subscription whose backlog the snapshot retains.
    /// Specifically, the created snapshot is guaranteed to retain:
    ///   (a) The existing backlog on the subscription. More precisely, this is
    ///       defined as the messages in the subscription's backlog that are
    ///       unacknowledged upon the successful completion of the
    ///       `CreateSnapshot` request; as well as:
    ///   (b) Any messages published to the subscription's topic following the
    ///       successful completion of the CreateSnapshot request.
    /// Format is `projects/{project}/subscriptions/{sub}`.
    #[prost(string, tag = "2")]
    pub subscription: ::prost::alloc::string::String,
    /// See [Creating and managing
    /// labels](<https://cloud.google.com/pubsub/docs/labels>).
    #[prost(map = "string, string", tag = "3")]
    pub labels: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// Request for the UpdateSnapshot method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateSnapshotRequest {
    /// Required. The updated snapshot object.
    #[prost(message, optional, tag = "1")]
    pub snapshot: ::core::option::Option<Snapshot>,
    /// Required. Indicates which fields in the provided snapshot to update.
    /// Must be specified and non-empty.
    #[prost(message, optional, tag = "2")]
    pub update_mask: ::core::option::Option<::prost_types::FieldMask>,
}
/// A snapshot resource. Snapshots are used in
/// \[Seek\](<https://cloud.google.com/pubsub/docs/replay-overview>)
/// operations, which allow you to manage message acknowledgments in bulk. That
/// is, you can set the acknowledgment state of messages in an existing
/// subscription to the state captured by a snapshot.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Snapshot {
    /// The name of the snapshot.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// The name of the topic from which this snapshot is retaining messages.
    #[prost(string, tag = "2")]
    pub topic: ::prost::alloc::string::String,
    /// The snapshot is guaranteed to exist up until this time.
    /// A newly-created snapshot expires no later than 7 days from the time of its
    /// creation. Its exact lifetime is determined at creation by the existing
    /// backlog in the source subscription. Specifically, the lifetime of the
    /// snapshot is `7 days - (age of oldest unacked message in the subscription)`.
    /// For example, consider a subscription whose oldest unacked message is 3 days
    /// old. If a snapshot is created from this subscription, the snapshot -- which
    /// will always capture this 3-day-old backlog as long as the snapshot
    /// exists -- will expire in 4 days. The service will refuse to create a
    /// snapshot that would expire in less than 1 hour after creation.
    #[prost(message, optional, tag = "3")]
    pub expire_time: ::core::option::Option<::prost_types::Timestamp>,
    /// See [Creating and managing labels]
    /// (<https://cloud.google.com/pubsub/docs/labels>).
    #[prost(map = "string, string", tag = "4")]
    pub labels: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// Request for the GetSnapshot method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSnapshotRequest {
    /// Required. The name of the snapshot to get.
    /// Format is `projects/{project}/snapshots/{snap}`.
    #[prost(string, tag = "1")]
    pub snapshot: ::prost::alloc::string::String,
}
/// Request for the `ListSnapshots` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSnapshotsRequest {
    /// Required. The name of the project in which to list snapshots.
    /// Format is `projects/{project-id}`.
    #[prost(string, tag = "1")]
    pub project: ::prost::alloc::string::String,
    /// Maximum number of snapshots to return.
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    /// The value returned by the last `ListSnapshotsResponse`; indicates that this
    /// is a continuation of a prior `ListSnapshots` call, and that the system
    /// should return the next page of data.
    #[prost(string, tag = "3")]
    pub page_token: ::prost::alloc::string::String,
}
/// Response for the `ListSnapshots` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListSnapshotsResponse {
    /// The resulting snapshots.
    #[prost(message, repeated, tag = "1")]
    pub snapshots: ::prost::alloc::vec::Vec<Snapshot>,
    /// If not empty, indicates that there may be more snapshot that match the
    /// request; this value should be passed in a new `ListSnapshotsRequest`.
    #[prost(string, tag = "2")]
    pub next_page_token: ::prost::alloc::string::String,
}
/// Request for the `DeleteSnapshot` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteSnapshotRequest {
    /// Required. The name of the snapshot to delete.
    /// Format is `projects/{project}/snapshots/{snap}`.
    #[prost(string, tag = "1")]
    pub snapshot: ::prost::alloc::string::String,
}
/// Request for the `Seek` method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeekRequest {
    /// Required. The subscription to affect.
    #[prost(string, tag = "1")]
    pub subscription: ::prost::alloc::string::String,
    #[prost(oneof = "seek_request::Target", tags = "2, 3")]
    pub target: ::core::option::Option<seek_request::Target>,
}
/// Nested message and enum types in `SeekRequest`.
pub mod seek_request {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Target {
        /// The time to seek to.
        /// Messages retained in the subscription that were published before this
        /// time are marked as acknowledged, and messages retained in the
        /// subscription that were published after this time are marked as
        /// unacknowledged. Note that this operation affects only those messages
        /// retained in the subscription (configured by the combination of
        /// `message_retention_duration` and `retain_acked_messages`). For example,
        /// if `time` corresponds to a point before the message retention
        /// window (or to a point before the system's notion of the subscription
        /// creation time), only retained messages will be marked as unacknowledged,
        /// and already-expunged messages will not be restored.
        #[prost(message, tag = "2")]
        Time(::prost_types::Timestamp),
        /// The snapshot to seek to. The snapshot's topic must be the same as that of
        /// the provided subscription.
        /// Format is `projects/{project}/snapshots/{snap}`.
        #[prost(string, tag = "3")]
        Snapshot(::prost::alloc::string::String),
    }
}
/// Response for the `Seek` method (this response is empty).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeekResponse {}
/// Generated client implementations.
pub mod publisher_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The service that an application uses to manipulate topics, and to send
    /// messages to a topic.
    #[derive(Debug, Clone)]
    pub struct PublisherClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl PublisherClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> PublisherClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> PublisherClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            PublisherClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Creates the given topic with the given name. See the [resource name rules]
        /// (https://cloud.google.com/pubsub/docs/admin#resource_names).
        pub async fn create_topic(
            &mut self,
            request: impl tonic::IntoRequest<super::Topic>,
        ) -> std::result::Result<tonic::Response<super::Topic>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/CreateTopic",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "CreateTopic"));
            self.inner.unary(req, path, codec).await
        }
        /// Updates an existing topic. Note that certain properties of a
        /// topic are not modifiable.
        pub async fn update_topic(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateTopicRequest>,
        ) -> std::result::Result<tonic::Response<super::Topic>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/UpdateTopic",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "UpdateTopic"));
            self.inner.unary(req, path, codec).await
        }
        /// Adds one or more messages to the topic. Returns `NOT_FOUND` if the topic
        /// does not exist.
        pub async fn publish(
            &mut self,
            request: impl tonic::IntoRequest<super::PublishRequest>,
        ) -> std::result::Result<
            tonic::Response<super::PublishResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/Publish",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "Publish"));
            self.inner.unary(req, path, codec).await
        }
        /// Gets the configuration of a topic.
        pub async fn get_topic(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTopicRequest>,
        ) -> std::result::Result<tonic::Response<super::Topic>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/GetTopic",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "GetTopic"));
            self.inner.unary(req, path, codec).await
        }
        /// Lists matching topics.
        pub async fn list_topics(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTopicsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListTopicsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/ListTopics",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "ListTopics"));
            self.inner.unary(req, path, codec).await
        }
        /// Lists the names of the attached subscriptions on this topic.
        pub async fn list_topic_subscriptions(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTopicSubscriptionsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListTopicSubscriptionsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/ListTopicSubscriptions",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "google.pubsub.v1.Publisher",
                        "ListTopicSubscriptions",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Lists the names of the snapshots on this topic. Snapshots are used in
        /// [Seek](https://cloud.google.com/pubsub/docs/replay-overview) operations,
        /// which allow you to manage message acknowledgments in bulk. That is, you can
        /// set the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        pub async fn list_topic_snapshots(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTopicSnapshotsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListTopicSnapshotsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/ListTopicSnapshots",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Publisher", "ListTopicSnapshots"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Deletes the topic with the given name. Returns `NOT_FOUND` if the topic
        /// does not exist. After a topic is deleted, a new topic may be created with
        /// the same name; this is an entirely new topic with none of the old
        /// configuration or subscriptions. Existing subscriptions to this topic are
        /// not deleted, but their `topic` field is set to `_deleted-topic_`.
        pub async fn delete_topic(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteTopicRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/DeleteTopic",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Publisher", "DeleteTopic"));
            self.inner.unary(req, path, codec).await
        }
        /// Detaches a subscription from this topic. All messages retained in the
        /// subscription are dropped. Subsequent `Pull` and `StreamingPull` requests
        /// will return FAILED_PRECONDITION. If the subscription is a push
        /// subscription, pushes to the endpoint will stop.
        pub async fn detach_subscription(
            &mut self,
            request: impl tonic::IntoRequest<super::DetachSubscriptionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DetachSubscriptionResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Publisher/DetachSubscription",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Publisher", "DetachSubscription"),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod subscriber_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The service that an application uses to manipulate subscriptions and to
    /// consume messages from a subscription via the `Pull` method or by
    /// establishing a bi-directional stream using the `StreamingPull` method.
    #[derive(Debug, Clone)]
    pub struct SubscriberClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SubscriberClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SubscriberClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SubscriberClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SubscriberClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Creates a subscription to a given topic. See the [resource name rules]
        /// (https://cloud.google.com/pubsub/docs/admin#resource_names).
        /// If the subscription already exists, returns `ALREADY_EXISTS`.
        /// If the corresponding topic doesn't exist, returns `NOT_FOUND`.
        ///
        /// If the name is not provided in the request, the server will assign a random
        /// name for this subscription on the same project as the topic, conforming
        /// to the [resource name format]
        /// (https://cloud.google.com/pubsub/docs/admin#resource_names). The generated
        /// name is populated in the returned Subscription object. Note that for REST
        /// API requests, you must specify a name in the request.
        pub async fn create_subscription(
            &mut self,
            request: impl tonic::IntoRequest<super::Subscription>,
        ) -> std::result::Result<tonic::Response<super::Subscription>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/CreateSubscription",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "CreateSubscription"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Gets the configuration details of a subscription.
        pub async fn get_subscription(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSubscriptionRequest>,
        ) -> std::result::Result<tonic::Response<super::Subscription>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/GetSubscription",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "GetSubscription"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Updates an existing subscription. Note that certain properties of a
        /// subscription, such as its topic, are not modifiable.
        pub async fn update_subscription(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateSubscriptionRequest>,
        ) -> std::result::Result<tonic::Response<super::Subscription>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/UpdateSubscription",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "UpdateSubscription"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Lists matching subscriptions.
        pub async fn list_subscriptions(
            &mut self,
            request: impl tonic::IntoRequest<super::ListSubscriptionsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListSubscriptionsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/ListSubscriptions",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "ListSubscriptions"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Deletes an existing subscription. All messages retained in the subscription
        /// are immediately dropped. Calls to `Pull` after deletion will return
        /// `NOT_FOUND`. After a subscription is deleted, a new one may be created with
        /// the same name, but the new one has no association with the old
        /// subscription or its topic unless the same topic is specified.
        pub async fn delete_subscription(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteSubscriptionRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/DeleteSubscription",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "DeleteSubscription"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Modifies the ack deadline for a specific message. This method is useful
        /// to indicate that more time is needed to process a message by the
        /// subscriber, or to make the message available for redelivery if the
        /// processing was interrupted. Note that this does not modify the
        /// subscription-level `ackDeadlineSeconds` used for subsequent messages.
        pub async fn modify_ack_deadline(
            &mut self,
            request: impl tonic::IntoRequest<super::ModifyAckDeadlineRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/ModifyAckDeadline",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "ModifyAckDeadline"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Acknowledges the messages associated with the `ack_ids` in the
        /// `AcknowledgeRequest`. The Pub/Sub system can remove the relevant messages
        /// from the subscription.
        ///
        /// Acknowledging a message whose ack deadline has expired may succeed,
        /// but such a message may be redelivered later. Acknowledging a message more
        /// than once will not result in an error.
        pub async fn acknowledge(
            &mut self,
            request: impl tonic::IntoRequest<super::AcknowledgeRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/Acknowledge",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "Acknowledge"));
            self.inner.unary(req, path, codec).await
        }
        /// Pulls messages from the server.
        pub async fn pull(
            &mut self,
            request: impl tonic::IntoRequest<super::PullRequest>,
        ) -> std::result::Result<tonic::Response<super::PullResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/Pull",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "Pull"));
            self.inner.unary(req, path, codec).await
        }
        /// Establishes a stream with the server, which sends messages down to the
        /// client. The client streams acknowledgements and ack deadline modifications
        /// back to the server. The server will close the stream and return the status
        /// on any error. The server may close the stream with status `UNAVAILABLE` to
        /// reassign server-side resources, in which case, the client should
        /// re-establish the stream. Flow control can be achieved by configuring the
        /// underlying RPC channel.
        pub async fn streaming_pull(
            &mut self,
            request: impl tonic::IntoStreamingRequest<
                Message = super::StreamingPullRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::StreamingPullResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/StreamingPull",
            );
            let mut req = request.into_streaming_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "StreamingPull"));
            self.inner.streaming(req, path, codec).await
        }
        /// Modifies the `PushConfig` for a specified subscription.
        ///
        /// This may be used to change a push subscription to a pull one (signified by
        /// an empty `PushConfig`) or vice versa, or change the endpoint URL and other
        /// attributes of a push subscription. Messages will accumulate for delivery
        /// continuously through the call regardless of changes to the `PushConfig`.
        pub async fn modify_push_config(
            &mut self,
            request: impl tonic::IntoRequest<super::ModifyPushConfigRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/ModifyPushConfig",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "ModifyPushConfig"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Gets the configuration details of a snapshot. Snapshots are used in
        /// [Seek](https://cloud.google.com/pubsub/docs/replay-overview) operations,
        /// which allow you to manage message acknowledgments in bulk. That is, you can
        /// set the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        pub async fn get_snapshot(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSnapshotRequest>,
        ) -> std::result::Result<tonic::Response<super::Snapshot>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/GetSnapshot",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "GetSnapshot"));
            self.inner.unary(req, path, codec).await
        }
        /// Lists the existing snapshots. Snapshots are used in [Seek](
        /// https://cloud.google.com/pubsub/docs/replay-overview) operations, which
        /// allow you to manage message acknowledgments in bulk. That is, you can set
        /// the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        pub async fn list_snapshots(
            &mut self,
            request: impl tonic::IntoRequest<super::ListSnapshotsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ListSnapshotsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/ListSnapshots",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "ListSnapshots"));
            self.inner.unary(req, path, codec).await
        }
        /// Creates a snapshot from the requested subscription. Snapshots are used in
        /// [Seek](https://cloud.google.com/pubsub/docs/replay-overview) operations,
        /// which allow you to manage message acknowledgments in bulk. That is, you can
        /// set the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        /// If the snapshot already exists, returns `ALREADY_EXISTS`.
        /// If the requested subscription doesn't exist, returns `NOT_FOUND`.
        /// If the backlog in the subscription is too old -- and the resulting snapshot
        /// would expire in less than 1 hour -- then `FAILED_PRECONDITION` is returned.
        /// See also the `Snapshot.expire_time` field. If the name is not provided in
        /// the request, the server will assign a random
        /// name for this snapshot on the same project as the subscription, conforming
        /// to the [resource name format]
        /// (https://cloud.google.com/pubsub/docs/admin#resource_names). The
        /// generated name is populated in the returned Snapshot object. Note that for
        /// REST API requests, you must specify a name in the request.
        pub async fn create_snapshot(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateSnapshotRequest>,
        ) -> std::result::Result<tonic::Response<super::Snapshot>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/CreateSnapshot",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "CreateSnapshot"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Updates an existing snapshot. Snapshots are used in
        /// [Seek](https://cloud.google.com/pubsub/docs/replay-overview) operations,
        /// which allow you to manage message acknowledgments in bulk. That is, you can
        /// set the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        pub async fn update_snapshot(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateSnapshotRequest>,
        ) -> std::result::Result<tonic::Response<super::Snapshot>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/UpdateSnapshot",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "UpdateSnapshot"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Removes an existing snapshot. Snapshots are used in [Seek]
        /// (https://cloud.google.com/pubsub/docs/replay-overview) operations, which
        /// allow you to manage message acknowledgments in bulk. That is, you can set
        /// the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot.
        /// When the snapshot is deleted, all messages retained in the snapshot
        /// are immediately dropped. After a snapshot is deleted, a new one may be
        /// created with the same name, but the new one has no association with the old
        /// snapshot or its subscription, unless the same subscription is specified.
        pub async fn delete_snapshot(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteSnapshotRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/DeleteSnapshot",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("google.pubsub.v1.Subscriber", "DeleteSnapshot"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Seeks an existing subscription to a point in time or to a given snapshot,
        /// whichever is provided in the request. Snapshots are used in [Seek]
        /// (https://cloud.google.com/pubsub/docs/replay-overview) operations, which
        /// allow you to manage message acknowledgments in bulk. That is, you can set
        /// the acknowledgment state of messages in an existing subscription to the
        /// state captured by a snapshot. Note that both the subscription and the
        /// snapshot must be on the same topic.
        pub async fn seek(
            &mut self,
            request: impl tonic::IntoRequest<super::SeekRequest>,
        ) -> std::result::Result<tonic::Response<super::SeekResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/google.pubsub.v1.Subscriber/Seek",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("google.pubsub.v1.Subscriber", "Seek"));
            self.inner.unary(req, path, codec).await
        }
    }
}
