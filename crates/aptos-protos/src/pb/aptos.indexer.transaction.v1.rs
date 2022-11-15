// @generated
/// Transaction is a simplified representation for the transaction
/// happened on the chain. Mainly built for streaming into BigQuery.
/// It matches with the structure defined for the transaction in Indexer.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transaction {
    #[prost(int64, required, tag="1")]
    pub version: i64,
    #[prost(int64, required, tag="2")]
    pub block_height: i64,
    #[prost(string, required, tag="3")]
    pub hash: ::prost::alloc::string::String,
    #[prost(string, required, tag="4")]
    pub type_str: ::prost::alloc::string::String,
    #[prost(string, optional, tag="5")]
    pub payload: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, required, tag="6")]
    pub state_change_hash: ::prost::alloc::string::String,
    #[prost(string, required, tag="7")]
    pub event_root_hash: ::prost::alloc::string::String,
    #[prost(string, optional, tag="8")]
    pub state_checkpoint_hash: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(uint64, required, tag="9")]
    pub gas_used: u64,
    #[prost(bool, required, tag="10")]
    pub success: bool,
    #[prost(string, required, tag="11")]
    pub vm_status: ::prost::alloc::string::String,
    #[prost(string, required, tag="12")]
    pub accumulator_root_hash: ::prost::alloc::string::String,
    #[prost(int64, required, tag="13")]
    pub num_events: i64,
    #[prost(int64, required, tag="14")]
    pub num_write_set_changes: i64,
    #[prost(int64, required, tag="15")]
    pub epoch: i64,
    #[prost(int64, required, tag="16")]
    pub inserted_at: i64,
}
/// Encoded file descriptor set for the `aptos.indexer.transaction.v1` package
pub const FILE_DESCRIPTOR_SET: &[u8] = &[
    0x0a, 0x8d, 0x12, 0x0a, 0x26, 0x61, 0x70, 0x74, 0x6f, 0x73, 0x2f, 0x74, 0x72, 0x61, 0x6e, 0x73,
    0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x2f, 0x76, 0x31, 0x2f, 0x74, 0x72, 0x61, 0x6e, 0x73, 0x61,
    0x63, 0x74, 0x69, 0x6f, 0x6e, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x1c, 0x61, 0x70, 0x74,
    0x6f, 0x73, 0x2e, 0x69, 0x6e, 0x64, 0x65, 0x78, 0x65, 0x72, 0x2e, 0x74, 0x72, 0x61, 0x6e, 0x73,
    0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x2e, 0x76, 0x31, 0x22, 0xaa, 0x04, 0x0a, 0x0b, 0x54, 0x72,
    0x61, 0x6e, 0x73, 0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x12, 0x18, 0x0a, 0x07, 0x76, 0x65, 0x72,
    0x73, 0x69, 0x6f, 0x6e, 0x18, 0x01, 0x20, 0x02, 0x28, 0x03, 0x52, 0x07, 0x76, 0x65, 0x72, 0x73,
    0x69, 0x6f, 0x6e, 0x12, 0x21, 0x0a, 0x0c, 0x62, 0x6c, 0x6f, 0x63, 0x6b, 0x5f, 0x68, 0x65, 0x69,
    0x67, 0x68, 0x74, 0x18, 0x02, 0x20, 0x02, 0x28, 0x03, 0x52, 0x0b, 0x62, 0x6c, 0x6f, 0x63, 0x6b,
    0x48, 0x65, 0x69, 0x67, 0x68, 0x74, 0x12, 0x12, 0x0a, 0x04, 0x68, 0x61, 0x73, 0x68, 0x18, 0x03,
    0x20, 0x02, 0x28, 0x09, 0x52, 0x04, 0x68, 0x61, 0x73, 0x68, 0x12, 0x19, 0x0a, 0x08, 0x74, 0x79,
    0x70, 0x65, 0x5f, 0x73, 0x74, 0x72, 0x18, 0x04, 0x20, 0x02, 0x28, 0x09, 0x52, 0x07, 0x74, 0x79,
    0x70, 0x65, 0x53, 0x74, 0x72, 0x12, 0x18, 0x0a, 0x07, 0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64,
    0x18, 0x05, 0x20, 0x01, 0x28, 0x09, 0x52, 0x07, 0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64, 0x12,
    0x2a, 0x0a, 0x11, 0x73, 0x74, 0x61, 0x74, 0x65, 0x5f, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x5f,
    0x68, 0x61, 0x73, 0x68, 0x18, 0x06, 0x20, 0x02, 0x28, 0x09, 0x52, 0x0f, 0x73, 0x74, 0x61, 0x74,
    0x65, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x48, 0x61, 0x73, 0x68, 0x12, 0x26, 0x0a, 0x0f, 0x65,
    0x76, 0x65, 0x6e, 0x74, 0x5f, 0x72, 0x6f, 0x6f, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x18, 0x07,
    0x20, 0x02, 0x28, 0x09, 0x52, 0x0d, 0x65, 0x76, 0x65, 0x6e, 0x74, 0x52, 0x6f, 0x6f, 0x74, 0x48,
    0x61, 0x73, 0x68, 0x12, 0x32, 0x0a, 0x15, 0x73, 0x74, 0x61, 0x74, 0x65, 0x5f, 0x63, 0x68, 0x65,
    0x63, 0x6b, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x18, 0x08, 0x20, 0x01,
    0x28, 0x09, 0x52, 0x13, 0x73, 0x74, 0x61, 0x74, 0x65, 0x43, 0x68, 0x65, 0x63, 0x6b, 0x70, 0x6f,
    0x69, 0x6e, 0x74, 0x48, 0x61, 0x73, 0x68, 0x12, 0x19, 0x0a, 0x08, 0x67, 0x61, 0x73, 0x5f, 0x75,
    0x73, 0x65, 0x64, 0x18, 0x09, 0x20, 0x02, 0x28, 0x04, 0x52, 0x07, 0x67, 0x61, 0x73, 0x55, 0x73,
    0x65, 0x64, 0x12, 0x18, 0x0a, 0x07, 0x73, 0x75, 0x63, 0x63, 0x65, 0x73, 0x73, 0x18, 0x0a, 0x20,
    0x02, 0x28, 0x08, 0x52, 0x07, 0x73, 0x75, 0x63, 0x63, 0x65, 0x73, 0x73, 0x12, 0x1b, 0x0a, 0x09,
    0x76, 0x6d, 0x5f, 0x73, 0x74, 0x61, 0x74, 0x75, 0x73, 0x18, 0x0b, 0x20, 0x02, 0x28, 0x09, 0x52,
    0x08, 0x76, 0x6d, 0x53, 0x74, 0x61, 0x74, 0x75, 0x73, 0x12, 0x32, 0x0a, 0x15, 0x61, 0x63, 0x63,
    0x75, 0x6d, 0x75, 0x6c, 0x61, 0x74, 0x6f, 0x72, 0x5f, 0x72, 0x6f, 0x6f, 0x74, 0x5f, 0x68, 0x61,
    0x73, 0x68, 0x18, 0x0c, 0x20, 0x02, 0x28, 0x09, 0x52, 0x13, 0x61, 0x63, 0x63, 0x75, 0x6d, 0x75,
    0x6c, 0x61, 0x74, 0x6f, 0x72, 0x52, 0x6f, 0x6f, 0x74, 0x48, 0x61, 0x73, 0x68, 0x12, 0x1d, 0x0a,
    0x0a, 0x6e, 0x75, 0x6d, 0x5f, 0x65, 0x76, 0x65, 0x6e, 0x74, 0x73, 0x18, 0x0d, 0x20, 0x02, 0x28,
    0x03, 0x52, 0x09, 0x6e, 0x75, 0x6d, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x73, 0x12, 0x31, 0x0a, 0x15,
    0x6e, 0x75, 0x6d, 0x5f, 0x77, 0x72, 0x69, 0x74, 0x65, 0x5f, 0x73, 0x65, 0x74, 0x5f, 0x63, 0x68,
    0x61, 0x6e, 0x67, 0x65, 0x73, 0x18, 0x0e, 0x20, 0x02, 0x28, 0x03, 0x52, 0x12, 0x6e, 0x75, 0x6d,
    0x57, 0x72, 0x69, 0x74, 0x65, 0x53, 0x65, 0x74, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x73, 0x12,
    0x14, 0x0a, 0x05, 0x65, 0x70, 0x6f, 0x63, 0x68, 0x18, 0x0f, 0x20, 0x02, 0x28, 0x03, 0x52, 0x05,
    0x65, 0x70, 0x6f, 0x63, 0x68, 0x12, 0x1f, 0x0a, 0x0b, 0x69, 0x6e, 0x73, 0x65, 0x72, 0x74, 0x65,
    0x64, 0x5f, 0x61, 0x74, 0x18, 0x10, 0x20, 0x02, 0x28, 0x03, 0x52, 0x0a, 0x69, 0x6e, 0x73, 0x65,
    0x72, 0x74, 0x65, 0x64, 0x41, 0x74, 0x4a, 0x97, 0x0d, 0x0a, 0x06, 0x12, 0x04, 0x08, 0x00, 0x20,
    0x01, 0x0a, 0xcd, 0x02, 0x0a, 0x01, 0x0c, 0x12, 0x03, 0x08, 0x00, 0x12, 0x1a, 0x86, 0x02, 0x20,
    0x50, 0x72, 0x6f, 0x74, 0x6f, 0x32, 0x20, 0x69, 0x73, 0x20, 0x72, 0x65, 0x71, 0x75, 0x69, 0x72,
    0x65, 0x64, 0x2e, 0x0a, 0x20, 0x43, 0x75, 0x72, 0x72, 0x65, 0x6e, 0x74, 0x20, 0x42, 0x69, 0x67,
    0x51, 0x75, 0x65, 0x72, 0x79, 0x20, 0x72, 0x75, 0x6e, 0x73, 0x20, 0x6f, 0x76, 0x65, 0x72, 0x20,
    0x70, 0x72, 0x6f, 0x74, 0x6f, 0x32, 0x2c, 0x20, 0x74, 0x68, 0x75, 0x73, 0x20, 0x6f, 0x70, 0x74,
    0x69, 0x6f, 0x6e, 0x61, 0x6c, 0x28, 0x6e, 0x75, 0x6c, 0x6c, 0x61, 0x62, 0x6c, 0x65, 0x29, 0x0a,
    0x20, 0x66, 0x69, 0x65, 0x6c, 0x64, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x64, 0x65, 0x66, 0x61,
    0x75, 0x6c, 0x74, 0x20, 0x76, 0x61, 0x6c, 0x75, 0x65, 0x20, 0x77, 0x69, 0x6c, 0x6c, 0x20, 0x62,
    0x65, 0x20, 0x69, 0x67, 0x6e, 0x6f, 0x72, 0x65, 0x64, 0x2e, 0x20, 0x46, 0x6f, 0x72, 0x20, 0x65,
    0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2c, 0x0a, 0x20, 0x60, 0x69, 0x6e, 0x74, 0x36, 0x34, 0x20,
    0x76, 0x61, 0x6c, 0x75, 0x65, 0x20, 0x3d, 0x20, 0x30, 0x60, 0x20, 0x77, 0x69, 0x6c, 0x6c, 0x20,
    0x62, 0x65, 0x20, 0x74, 0x72, 0x61, 0x6e, 0x73, 0x6c, 0x61, 0x74, 0x65, 0x64, 0x20, 0x74, 0x6f,
    0x20, 0x6e, 0x75, 0x6c, 0x6c, 0x20, 0x75, 0x6e, 0x64, 0x65, 0x72, 0x20, 0x63, 0x6f, 0x6c, 0x75,
    0x6d, 0x6e, 0x20, 0x60, 0x76, 0x61, 0x6c, 0x75, 0x65, 0x60, 0x2e, 0x0a, 0x20, 0x54, 0x6f, 0x20,
    0x61, 0x76, 0x6f, 0x69, 0x64, 0x20, 0x61, 0x6e, 0x79, 0x20, 0x61, 0x6e, 0x61, 0x6c, 0x79, 0x74,
    0x69, 0x63, 0x73, 0x20, 0x68, 0x61, 0x73, 0x73, 0x6c, 0x65, 0x2c, 0x20, 0x70, 0x72, 0x6f, 0x74,
    0x6f, 0x32, 0x20, 0x69, 0x73, 0x20, 0x72, 0x65, 0x71, 0x75, 0x69, 0x72, 0x65, 0x64, 0x20, 0x68,
    0x65, 0x72, 0x65, 0x2e, 0x0a, 0x32, 0x3a, 0x20, 0x43, 0x6f, 0x70, 0x79, 0x72, 0x69, 0x67, 0x68,
    0x74, 0x20, 0x28, 0x63, 0x29, 0x20, 0x41, 0x70, 0x74, 0x6f, 0x73, 0x0a, 0x20, 0x53, 0x50, 0x44,
    0x58, 0x2d, 0x4c, 0x69, 0x63, 0x65, 0x6e, 0x73, 0x65, 0x2d, 0x49, 0x64, 0x65, 0x6e, 0x74, 0x69,
    0x66, 0x69, 0x65, 0x72, 0x3a, 0x20, 0x41, 0x70, 0x61, 0x63, 0x68, 0x65, 0x2d, 0x32, 0x2e, 0x30,
    0x0a, 0x0a, 0x08, 0x0a, 0x01, 0x02, 0x12, 0x03, 0x0a, 0x00, 0x25, 0x0a, 0xd6, 0x01, 0x0a, 0x02,
    0x04, 0x00, 0x12, 0x04, 0x0f, 0x00, 0x20, 0x01, 0x1a, 0xc9, 0x01, 0x20, 0x54, 0x72, 0x61, 0x6e,
    0x73, 0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x20, 0x69, 0x73, 0x20, 0x61, 0x20, 0x73, 0x69, 0x6d,
    0x70, 0x6c, 0x69, 0x66, 0x69, 0x65, 0x64, 0x20, 0x72, 0x65, 0x70, 0x72, 0x65, 0x73, 0x65, 0x6e,
    0x74, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x74, 0x68, 0x65, 0x20, 0x74,
    0x72, 0x61, 0x6e, 0x73, 0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x0a, 0x20, 0x68, 0x61, 0x70, 0x70,
    0x65, 0x6e, 0x65, 0x64, 0x20, 0x6f, 0x6e, 0x20, 0x74, 0x68, 0x65, 0x20, 0x63, 0x68, 0x61, 0x69,
    0x6e, 0x2e, 0x20, 0x4d, 0x61, 0x69, 0x6e, 0x6c, 0x79, 0x20, 0x62, 0x75, 0x69, 0x6c, 0x74, 0x20,
    0x66, 0x6f, 0x72, 0x20, 0x73, 0x74, 0x72, 0x65, 0x61, 0x6d, 0x69, 0x6e, 0x67, 0x20, 0x69, 0x6e,
    0x74, 0x6f, 0x20, 0x42, 0x69, 0x67, 0x51, 0x75, 0x65, 0x72, 0x79, 0x2e, 0x0a, 0x20, 0x49, 0x74,
    0x20, 0x6d, 0x61, 0x74, 0x63, 0x68, 0x65, 0x73, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x74, 0x68,
    0x65, 0x20, 0x73, 0x74, 0x72, 0x75, 0x63, 0x74, 0x75, 0x72, 0x65, 0x20, 0x64, 0x65, 0x66, 0x69,
    0x6e, 0x65, 0x64, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x74, 0x68, 0x65, 0x20, 0x74, 0x72, 0x61, 0x6e,
    0x73, 0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x20, 0x69, 0x6e, 0x20, 0x49, 0x6e, 0x64, 0x65, 0x78,
    0x65, 0x72, 0x2e, 0x0a, 0x0a, 0x0a, 0x0a, 0x03, 0x04, 0x00, 0x01, 0x12, 0x03, 0x0f, 0x08, 0x13,
    0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x00, 0x12, 0x03, 0x10, 0x04, 0x1f, 0x0a, 0x0c, 0x0a,
    0x05, 0x04, 0x00, 0x02, 0x00, 0x04, 0x12, 0x03, 0x10, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04,
    0x00, 0x02, 0x00, 0x05, 0x12, 0x03, 0x10, 0x0d, 0x12, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02,
    0x00, 0x01, 0x12, 0x03, 0x10, 0x13, 0x1a, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x00, 0x03,
    0x12, 0x03, 0x10, 0x1d, 0x1e, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x01, 0x12, 0x03, 0x11,
    0x04, 0x24, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x01, 0x04, 0x12, 0x03, 0x11, 0x04, 0x0c,
    0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x01, 0x05, 0x12, 0x03, 0x11, 0x0d, 0x12, 0x0a, 0x0c,
    0x0a, 0x05, 0x04, 0x00, 0x02, 0x01, 0x01, 0x12, 0x03, 0x11, 0x13, 0x1f, 0x0a, 0x0c, 0x0a, 0x05,
    0x04, 0x00, 0x02, 0x01, 0x03, 0x12, 0x03, 0x11, 0x22, 0x23, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00,
    0x02, 0x02, 0x12, 0x03, 0x12, 0x04, 0x1d, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x02, 0x04,
    0x12, 0x03, 0x12, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x02, 0x05, 0x12, 0x03,
    0x12, 0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x02, 0x01, 0x12, 0x03, 0x12, 0x14,
    0x18, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x02, 0x03, 0x12, 0x03, 0x12, 0x1b, 0x1c, 0x0a,
    0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x03, 0x12, 0x03, 0x13, 0x04, 0x21, 0x0a, 0x0c, 0x0a, 0x05,
    0x04, 0x00, 0x02, 0x03, 0x04, 0x12, 0x03, 0x13, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00,
    0x02, 0x03, 0x05, 0x12, 0x03, 0x13, 0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x03,
    0x01, 0x12, 0x03, 0x13, 0x14, 0x1c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x03, 0x03, 0x12,
    0x03, 0x13, 0x1f, 0x20, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x04, 0x12, 0x03, 0x14, 0x04,
    0x20, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x04, 0x04, 0x12, 0x03, 0x14, 0x04, 0x0c, 0x0a,
    0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x04, 0x05, 0x12, 0x03, 0x14, 0x0d, 0x13, 0x0a, 0x0c, 0x0a,
    0x05, 0x04, 0x00, 0x02, 0x04, 0x01, 0x12, 0x03, 0x14, 0x14, 0x1b, 0x0a, 0x0c, 0x0a, 0x05, 0x04,
    0x00, 0x02, 0x04, 0x03, 0x12, 0x03, 0x14, 0x1e, 0x1f, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02,
    0x05, 0x12, 0x03, 0x15, 0x04, 0x2a, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x05, 0x04, 0x12,
    0x03, 0x15, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x05, 0x05, 0x12, 0x03, 0x15,
    0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x05, 0x01, 0x12, 0x03, 0x15, 0x14, 0x25,
    0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x05, 0x03, 0x12, 0x03, 0x15, 0x28, 0x29, 0x0a, 0x0b,
    0x0a, 0x04, 0x04, 0x00, 0x02, 0x06, 0x12, 0x03, 0x16, 0x04, 0x28, 0x0a, 0x0c, 0x0a, 0x05, 0x04,
    0x00, 0x02, 0x06, 0x04, 0x12, 0x03, 0x16, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02,
    0x06, 0x05, 0x12, 0x03, 0x16, 0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x06, 0x01,
    0x12, 0x03, 0x16, 0x14, 0x23, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x06, 0x03, 0x12, 0x03,
    0x16, 0x26, 0x27, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x07, 0x12, 0x03, 0x17, 0x04, 0x2e,
    0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x07, 0x04, 0x12, 0x03, 0x17, 0x04, 0x0c, 0x0a, 0x0c,
    0x0a, 0x05, 0x04, 0x00, 0x02, 0x07, 0x05, 0x12, 0x03, 0x17, 0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05,
    0x04, 0x00, 0x02, 0x07, 0x01, 0x12, 0x03, 0x17, 0x14, 0x29, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00,
    0x02, 0x07, 0x03, 0x12, 0x03, 0x17, 0x2c, 0x2d, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x08,
    0x12, 0x03, 0x18, 0x04, 0x21, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x08, 0x04, 0x12, 0x03,
    0x18, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x08, 0x05, 0x12, 0x03, 0x18, 0x0d,
    0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x08, 0x01, 0x12, 0x03, 0x18, 0x14, 0x1c, 0x0a,
    0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x08, 0x03, 0x12, 0x03, 0x18, 0x1f, 0x20, 0x0a, 0x0b, 0x0a,
    0x04, 0x04, 0x00, 0x02, 0x09, 0x12, 0x03, 0x19, 0x04, 0x1f, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00,
    0x02, 0x09, 0x04, 0x12, 0x03, 0x19, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x09,
    0x05, 0x12, 0x03, 0x19, 0x0d, 0x11, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x09, 0x01, 0x12,
    0x03, 0x19, 0x12, 0x19, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x09, 0x03, 0x12, 0x03, 0x19,
    0x1c, 0x1e, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x0a, 0x12, 0x03, 0x1a, 0x04, 0x23, 0x0a,
    0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0a, 0x04, 0x12, 0x03, 0x1a, 0x04, 0x0c, 0x0a, 0x0c, 0x0a,
    0x05, 0x04, 0x00, 0x02, 0x0a, 0x05, 0x12, 0x03, 0x1a, 0x0d, 0x13, 0x0a, 0x0c, 0x0a, 0x05, 0x04,
    0x00, 0x02, 0x0a, 0x01, 0x12, 0x03, 0x1a, 0x14, 0x1d, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02,
    0x0a, 0x03, 0x12, 0x03, 0x1a, 0x20, 0x22, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x0b, 0x12,
    0x03, 0x1b, 0x04, 0x2f, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0b, 0x04, 0x12, 0x03, 0x1b,
    0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0b, 0x05, 0x12, 0x03, 0x1b, 0x0d, 0x13,
    0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0b, 0x01, 0x12, 0x03, 0x1b, 0x14, 0x29, 0x0a, 0x0c,
    0x0a, 0x05, 0x04, 0x00, 0x02, 0x0b, 0x03, 0x12, 0x03, 0x1b, 0x2c, 0x2e, 0x0a, 0x0b, 0x0a, 0x04,
    0x04, 0x00, 0x02, 0x0c, 0x12, 0x03, 0x1c, 0x04, 0x23, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02,
    0x0c, 0x04, 0x12, 0x03, 0x1c, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0c, 0x05,
    0x12, 0x03, 0x1c, 0x0d, 0x12, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0c, 0x01, 0x12, 0x03,
    0x1c, 0x13, 0x1d, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0c, 0x03, 0x12, 0x03, 0x1c, 0x20,
    0x22, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x0d, 0x12, 0x03, 0x1d, 0x04, 0x2e, 0x0a, 0x0c,
    0x0a, 0x05, 0x04, 0x00, 0x02, 0x0d, 0x04, 0x12, 0x03, 0x1d, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05,
    0x04, 0x00, 0x02, 0x0d, 0x05, 0x12, 0x03, 0x1d, 0x0d, 0x12, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00,
    0x02, 0x0d, 0x01, 0x12, 0x03, 0x1d, 0x13, 0x28, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0d,
    0x03, 0x12, 0x03, 0x1d, 0x2b, 0x2d, 0x0a, 0x0b, 0x0a, 0x04, 0x04, 0x00, 0x02, 0x0e, 0x12, 0x03,
    0x1e, 0x04, 0x1e, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0e, 0x04, 0x12, 0x03, 0x1e, 0x04,
    0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0e, 0x05, 0x12, 0x03, 0x1e, 0x0d, 0x12, 0x0a,
    0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0e, 0x01, 0x12, 0x03, 0x1e, 0x13, 0x18, 0x0a, 0x0c, 0x0a,
    0x05, 0x04, 0x00, 0x02, 0x0e, 0x03, 0x12, 0x03, 0x1e, 0x1b, 0x1d, 0x0a, 0x0b, 0x0a, 0x04, 0x04,
    0x00, 0x02, 0x0f, 0x12, 0x03, 0x1f, 0x04, 0x24, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0f,
    0x04, 0x12, 0x03, 0x1f, 0x04, 0x0c, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0f, 0x05, 0x12,
    0x03, 0x1f, 0x0d, 0x12, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0f, 0x01, 0x12, 0x03, 0x1f,
    0x13, 0x1e, 0x0a, 0x0c, 0x0a, 0x05, 0x04, 0x00, 0x02, 0x0f, 0x03, 0x12, 0x03, 0x1f, 0x21, 0x23,
];
// @@protoc_insertion_point(module)