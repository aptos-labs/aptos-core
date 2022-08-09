// @generated
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockOutput {
    #[prost(message, repeated, tag="1")]
    pub transactions: ::prost::alloc::vec::Vec<TransactionOutput>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionOutput {
    #[prost(message, optional, tag="1")]
    pub transaction_info_output: ::core::option::Option<TransactionInfoOutput>,
    #[prost(message, repeated, tag="4")]
    pub events: ::prost::alloc::vec::Vec<EventOutput>,
    #[prost(message, repeated, tag="5")]
    pub write_set_changes: ::prost::alloc::vec::Vec<WriteSetChangeOutput>,
    #[prost(oneof="transaction_output::TxnData", tags="2, 3")]
    pub txn_data: ::core::option::Option<transaction_output::TxnData>,
}
/// Nested message and enum types in `TransactionOutput`.
pub mod transaction_output {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum TxnData {
        #[prost(message, tag="2")]
        BlockMetadata(super::BlockMetadataTransactionOutput),
        #[prost(message, tag="3")]
        User(super::UserTransactionOutput),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionInfoOutput {
    #[prost(bytes="vec", tag="1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub r#type: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub version: u64,
    #[prost(bytes="vec", tag="4")]
    pub state_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="5")]
    pub event_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="6")]
    pub gas_used: u64,
    #[prost(bool, tag="7")]
    pub success: bool,
    #[prost(uint64, tag="8")]
    pub epoch: u64,
    #[prost(uint64, tag="9")]
    pub block_height: u64,
    #[prost(string, tag="10")]
    pub vm_status: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="11")]
    pub accumulator_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="12")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockMetadataTransactionOutput {
    #[prost(bytes="vec", tag="1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub round: u64,
    #[prost(bool, repeated, tag="4")]
    pub previous_block_votes_bitmap: ::prost::alloc::vec::Vec<bool>,
    #[prost(string, tag="5")]
    pub proposer: ::prost::alloc::string::String,
    #[prost(uint32, repeated, tag="6")]
    pub failed_proposer_indices: ::prost::alloc::vec::Vec<u32>,
    #[prost(message, optional, tag="7")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(uint64, tag="8")]
    pub epoch: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserTransactionOutput {
    #[prost(bytes="vec", tag="1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub parent_signature_type: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub sender: ::prost::alloc::string::String,
    #[prost(uint64, tag="4")]
    pub sequence_number: u64,
    #[prost(uint64, tag="5")]
    pub max_gas_amount: u64,
    #[prost(message, optional, tag="6")]
    pub expiration_timestamp_secs: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(uint64, tag="7")]
    pub gas_unit_price: u64,
    #[prost(message, optional, tag="8")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(message, repeated, tag="9")]
    pub signatures: ::prost::alloc::vec::Vec<SignatureOutput>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignatureOutput {
    #[prost(bytes="vec", tag="1")]
    pub transaction_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub signer: ::prost::alloc::string::String,
    #[prost(bool, tag="3")]
    pub is_sender_primary: bool,
    #[prost(string, tag="4")]
    pub signature_type: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="5")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="6")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag="7")]
    pub threshold: u32,
    #[prost(bytes="vec", tag="8")]
    pub bitmap: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag="9")]
    pub multi_agent_index: u32,
    #[prost(uint32, tag="10")]
    pub multi_sig_index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EventOutput {
    #[prost(bytes="vec", tag="1")]
    pub transaction_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub key: ::core::option::Option<EventKeyOutput>,
    #[prost(uint64, tag="3")]
    pub sequence_number: u64,
    #[prost(string, tag="4")]
    pub move_type: ::prost::alloc::string::String,
    #[prost(string, tag="5")]
    pub data: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EventKeyOutput {
    #[prost(uint64, tag="1")]
    pub creation_number: u64,
    #[prost(string, tag="2")]
    pub account_address: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteSetChangeOutput {
    #[prost(bytes="vec", tag="1")]
    pub transaction_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="3")]
    pub r#type: ::prost::alloc::string::String,
    #[prost(oneof="write_set_change_output::Change", tags="4, 5, 6")]
    pub change: ::core::option::Option<write_set_change_output::Change>,
}
/// Nested message and enum types in `WriteSetChangeOutput`.
pub mod write_set_change_output {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Change {
        #[prost(message, tag="4")]
        MoveModule(super::MoveModuleOutput),
        #[prost(message, tag="5")]
        MoveResource(super::MoveResourceOutput),
        #[prost(message, tag="6")]
        TableItem(super::TableItemOutput),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveModuleOutput {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="3")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="4")]
    pub bytecode: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, repeated, tag="5")]
    pub friends: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// this can be better
    #[prost(string, repeated, tag="6")]
    pub exposed_functions: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, repeated, tag="7")]
    pub structs: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(bool, tag="8")]
    pub is_deleted: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveResourceOutput {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub module: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, repeated, tag="4")]
    pub generic_type_params: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, tag="5")]
    pub data: ::prost::alloc::string::String,
    #[prost(bool, tag="6")]
    pub is_deleted: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TableItemOutput {
    #[prost(bytes="vec", tag="1")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub handle: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub decoded_key: ::prost::alloc::string::String,
    #[prost(string, tag="5")]
    pub key_type: ::prost::alloc::string::String,
    #[prost(string, tag="6")]
    pub decoded_value: ::prost::alloc::string::String,
    #[prost(string, tag="7")]
    pub value_type: ::prost::alloc::string::String,
    #[prost(bool, tag="8")]
    pub is_deleted: bool,
}
// @@protoc_insertion_point(module)
