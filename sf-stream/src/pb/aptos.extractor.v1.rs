#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transaction {
    #[prost(message, optional, tag="1")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(uint64, tag="2")]
    pub version: u64,
    #[prost(message, optional, tag="3")]
    pub info: ::core::option::Option<TransactionInfo>,
    #[prost(uint64, tag="4")]
    pub epoch: u64,
    #[prost(uint64, tag="5")]
    pub block_height: u64,
    #[prost(enumeration="transaction::TransactionType", tag="6")]
    pub r#type: i32,
    #[prost(oneof="transaction::TxnData", tags="7, 8, 9, 10")]
    pub txn_data: ::core::option::Option<transaction::TxnData>,
}
/// Nested message and enum types in `Transaction`.
pub mod transaction {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TransactionType {
        Genesis = 0,
        BlockMetadata = 1,
        StateCheckpoint = 2,
        User = 3,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum TxnData {
        #[prost(message, tag="7")]
        BlockMetadata(super::BlockMetadataTransaction),
        #[prost(message, tag="8")]
        Genesis(super::GenesisTransaction),
        #[prost(message, tag="9")]
        StateCheckpoint(super::StateCheckpointTransaction),
        #[prost(message, tag="10")]
        User(super::UserTransaction),
    }
}
/// TransactionTrimmed is a real Transaction with most of the fields removed so that
/// we can easily decode only the few fields that we have interest in in certain situations.
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionTrimmed {
    #[prost(message, optional, tag="1")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(uint64, tag="2")]
    pub version: u64,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockMetadataTransaction {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(uint64, tag="2")]
    pub round: u64,
    #[prost(message, repeated, tag="3")]
    pub events: ::prost::alloc::vec::Vec<Event>,
    #[prost(bool, repeated, tag="4")]
    pub previous_block_votes: ::prost::alloc::vec::Vec<bool>,
    #[prost(string, tag="5")]
    pub proposer: ::prost::alloc::string::String,
    #[prost(uint32, repeated, tag="6")]
    pub failed_proposer_indices: ::prost::alloc::vec::Vec<u32>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenesisTransaction {
    #[prost(message, optional, tag="1")]
    pub payload: ::core::option::Option<WriteSet>,
    #[prost(message, repeated, tag="2")]
    pub events: ::prost::alloc::vec::Vec<Event>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StateCheckpointTransaction {
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserTransaction {
    #[prost(message, optional, tag="1")]
    pub request: ::core::option::Option<UserTransactionRequest>,
    #[prost(message, repeated, tag="2")]
    pub events: ::prost::alloc::vec::Vec<Event>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Event {
    #[prost(message, optional, tag="1")]
    pub key: ::core::option::Option<EventKey>,
    #[prost(uint64, tag="2")]
    pub sequence_number: u64,
    #[prost(message, optional, tag="3")]
    pub r#type: ::core::option::Option<MoveType>,
    #[prost(string, tag="4")]
    pub data: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionInfo {
    #[prost(bytes="vec", tag="1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub state_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub event_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="4")]
    pub gas_used: u64,
    #[prost(bool, tag="5")]
    pub success: bool,
    #[prost(string, tag="6")]
    pub vm_status: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="7")]
    pub accumulator_root_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag="8")]
    pub changes: ::prost::alloc::vec::Vec<WriteSetChange>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EventKey {
    #[prost(uint64, tag="1")]
    pub creation_number: u64,
    #[prost(string, tag="2")]
    pub account_address: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserTransactionRequest {
    #[prost(string, tag="1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(uint64, tag="2")]
    pub sequence_number: u64,
    #[prost(uint64, tag="3")]
    pub max_gas_amount: u64,
    #[prost(uint64, tag="4")]
    pub gas_unit_price: u64,
    #[prost(message, optional, tag="5")]
    pub expiration_timestamp_secs: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(message, optional, tag="6")]
    pub payload: ::core::option::Option<TransactionPayload>,
    #[prost(message, optional, tag="7")]
    pub signature: ::core::option::Option<Signature>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteSet {
    #[prost(enumeration="write_set::WriteSetType", tag="1")]
    pub write_set_type: i32,
    #[prost(oneof="write_set::WriteSet", tags="2, 3")]
    pub write_set: ::core::option::Option<write_set::WriteSet>,
}
/// Nested message and enum types in `WriteSet`.
pub mod write_set {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum WriteSetType {
        ScriptWriteSet = 0,
        DirectWriteSet = 1,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum WriteSet {
        #[prost(message, tag="2")]
        ScriptWriteSet(super::ScriptWriteSet),
        #[prost(message, tag="3")]
        DirectWriteSet(super::DirectWriteSet),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScriptWriteSet {
    #[prost(string, tag="1")]
    pub execute_as: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub script: ::core::option::Option<ScriptPayload>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DirectWriteSet {
    #[prost(message, repeated, tag="1")]
    pub write_set_change: ::prost::alloc::vec::Vec<WriteSetChange>,
    #[prost(message, repeated, tag="2")]
    pub events: ::prost::alloc::vec::Vec<Event>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteSetChange {
    #[prost(enumeration="write_set_change::Type", tag="1")]
    pub r#type: i32,
    #[prost(oneof="write_set_change::Change", tags="2, 3, 4, 5, 6, 7")]
    pub change: ::core::option::Option<write_set_change::Change>,
}
/// Nested message and enum types in `WriteSetChange`.
pub mod write_set_change {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        DeleteModule = 0,
        DeleteResource = 1,
        DeleteTableItem = 2,
        WriteModule = 3,
        WriteResource = 4,
        WriteTableItem = 5,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Change {
        #[prost(message, tag="2")]
        DeleteModule(super::DeleteModule),
        #[prost(message, tag="3")]
        DeleteResource(super::DeleteResource),
        #[prost(message, tag="4")]
        DeleteTableItem(super::DeleteTableItem),
        #[prost(message, tag="5")]
        WriteModule(super::WriteModule),
        #[prost(message, tag="6")]
        WriteResource(super::WriteResource),
        #[prost(message, tag="7")]
        WriteTableItem(super::WriteTableItem),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteModule {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub module: ::core::option::Option<MoveModuleId>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteResource {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub resource: ::core::option::Option<MoveStructTag>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTableItem {
    #[prost(bytes="vec", tag="1")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub handle: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub data: ::core::option::Option<DeleteTableData>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTableData {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub key_type: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteModule {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub data: ::core::option::Option<MoveModuleBytecode>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteResource {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub data: ::core::option::Option<MoveResource>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteTableData {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub key_type: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub value: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub value_type: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteTableItem {
    #[prost(bytes="vec", tag="1")]
    pub state_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="2")]
    pub handle: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub data: ::core::option::Option<WriteTableData>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionPayload {
    #[prost(enumeration="transaction_payload::Type", tag="1")]
    pub r#type: i32,
    #[prost(oneof="transaction_payload::Payload", tags="2, 3, 4, 5")]
    pub payload: ::core::option::Option<transaction_payload::Payload>,
}
/// Nested message and enum types in `TransactionPayload`.
pub mod transaction_payload {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        ScriptFunctionPayload = 0,
        ScriptPayload = 1,
        ModuleBundlePayload = 2,
        WriteSetPayload = 3,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag="2")]
        ScriptFunctionPayload(super::ScriptFunctionPayload),
        #[prost(message, tag="3")]
        ScriptPayload(super::ScriptPayload),
        #[prost(message, tag="4")]
        ModuleBundlePayload(super::ModuleBundlePayload),
        #[prost(message, tag="5")]
        WriteSetPayload(super::WriteSetPayload),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScriptFunctionPayload {
    #[prost(message, optional, tag="1")]
    pub function: ::core::option::Option<ScriptFunctionId>,
    #[prost(message, repeated, tag="2")]
    pub type_arguments: ::prost::alloc::vec::Vec<MoveType>,
    #[prost(string, repeated, tag="3")]
    pub arguments: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveScriptBytecode {
    #[prost(bytes="vec", tag="1")]
    pub bytecode: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub abi: ::core::option::Option<MoveFunction>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScriptPayload {
    #[prost(message, optional, tag="1")]
    pub code: ::core::option::Option<MoveScriptBytecode>,
    #[prost(message, repeated, tag="2")]
    pub type_arguments: ::prost::alloc::vec::Vec<MoveType>,
    #[prost(string, repeated, tag="3")]
    pub arguments: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModuleBundlePayload {
    #[prost(message, repeated, tag="1")]
    pub modules: ::prost::alloc::vec::Vec<MoveModuleBytecode>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveModuleBytecode {
    #[prost(bytes="vec", tag="1")]
    pub bytecode: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub abi: ::core::option::Option<MoveModule>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveModule {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag="3")]
    pub friends: ::prost::alloc::vec::Vec<MoveModuleId>,
    #[prost(message, repeated, tag="4")]
    pub exposed_functions: ::prost::alloc::vec::Vec<MoveFunction>,
    #[prost(message, repeated, tag="5")]
    pub structs: ::prost::alloc::vec::Vec<MoveStruct>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveFunction {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(enumeration="move_function::Visibility", tag="2")]
    pub visibility: i32,
    #[prost(bool, tag="3")]
    pub is_entry: bool,
    #[prost(message, repeated, tag="4")]
    pub generic_type_params: ::prost::alloc::vec::Vec<MoveFunctionGenericTypeParam>,
    #[prost(message, repeated, tag="5")]
    pub params: ::prost::alloc::vec::Vec<MoveType>,
    #[prost(message, repeated, tag="6")]
    pub r#return: ::prost::alloc::vec::Vec<MoveType>,
}
/// Nested message and enum types in `MoveFunction`.
pub mod move_function {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Visibility {
        Private = 0,
        Public = 1,
        Friend = 2,
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveStruct {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(bool, tag="2")]
    pub is_native: bool,
    #[prost(enumeration="MoveAbility", repeated, tag="3")]
    pub abilities: ::prost::alloc::vec::Vec<i32>,
    #[prost(message, repeated, tag="4")]
    pub generic_type_params: ::prost::alloc::vec::Vec<MoveStructGenericTypeParam>,
    #[prost(message, repeated, tag="5")]
    pub fields: ::prost::alloc::vec::Vec<MoveStructField>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveStructGenericTypeParam {
    #[prost(enumeration="MoveAbility", repeated, tag="1")]
    pub constraints: ::prost::alloc::vec::Vec<i32>,
    #[prost(bool, tag="2")]
    pub is_phantom: bool,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveStructField {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub r#type: ::core::option::Option<MoveType>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveFunctionGenericTypeParam {
    #[prost(enumeration="MoveAbility", repeated, tag="1")]
    pub constraints: ::prost::alloc::vec::Vec<i32>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveType {
    #[prost(enumeration="MoveTypes", tag="1")]
    pub r#type: i32,
    #[prost(oneof="move_type::Content", tags="3, 4, 5, 6, 7")]
    pub content: ::core::option::Option<move_type::Content>,
}
/// Nested message and enum types in `MoveType`.
pub mod move_type {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ReferenceType {
        #[prost(bool, tag="1")]
        pub mutable: bool,
        #[prost(message, optional, boxed, tag="2")]
        pub to: ::core::option::Option<::prost::alloc::boxed::Box<super::MoveType>>,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(message, tag="3")]
        Vector(::prost::alloc::boxed::Box<super::MoveType>),
        #[prost(message, tag="4")]
        Struct(super::MoveStructTag),
        #[prost(uint32, tag="5")]
        GenericTypeParamIndex(u32),
        #[prost(message, tag="6")]
        Reference(::prost::alloc::boxed::Box<ReferenceType>),
        #[prost(string, tag="7")]
        Unparsable(::prost::alloc::string::String),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteSetPayload {
    #[prost(message, optional, tag="1")]
    pub write_set: ::core::option::Option<WriteSet>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScriptFunctionId {
    #[prost(message, optional, tag="1")]
    pub module: ::core::option::Option<MoveModuleId>,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveResource {
    #[prost(message, optional, tag="1")]
    pub r#type: ::core::option::Option<MoveStructTag>,
    #[prost(string, tag="2")]
    pub data: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveModuleId {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MoveStructTag {
    #[prost(string, tag="1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub module: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag="4")]
    pub generic_type_params: ::prost::alloc::vec::Vec<MoveType>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Signature {
    #[prost(enumeration="signature::Type", tag="1")]
    pub r#type: i32,
    #[prost(oneof="signature::Signature", tags="2, 3, 4")]
    pub signature: ::core::option::Option<signature::Signature>,
}
/// Nested message and enum types in `Signature`.
pub mod signature {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        Ed25519 = 0,
        MultiEd25519 = 1,
        MultiAgent = 2,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Signature {
        #[prost(message, tag="2")]
        Ed25519(super::Ed25519Signature),
        #[prost(message, tag="3")]
        MultiEd25519(super::MultiEd25519Signature),
        #[prost(message, tag="4")]
        MultiAgent(super::MultiAgentSignature),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ed25519Signature {
    #[prost(bytes="vec", tag="1")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MultiEd25519Signature {
    #[prost(bytes="vec", repeated, tag="1")]
    pub public_keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bytes="vec", repeated, tag="2")]
    pub signatures: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(uint32, tag="3")]
    pub threshold: u32,
    #[prost(bytes="vec", tag="4")]
    pub bitmap: ::prost::alloc::vec::Vec<u8>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MultiAgentSignature {
    #[prost(message, optional, tag="1")]
    pub sender: ::core::option::Option<AccountSignature>,
    #[prost(string, repeated, tag="2")]
    pub secondary_signer_addresses: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, repeated, tag="3")]
    pub secondary_signers: ::prost::alloc::vec::Vec<AccountSignature>,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountSignature {
    #[prost(enumeration="account_signature::Type", tag="1")]
    pub r#type: i32,
    #[prost(oneof="account_signature::Signature", tags="2, 3")]
    pub signature: ::core::option::Option<account_signature::Signature>,
}
/// Nested message and enum types in `AccountSignature`.
pub mod account_signature {
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        Ed25519 = 0,
        MultiEd25519 = 1,
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Signature {
        #[prost(message, tag="2")]
        Ed25519(super::Ed25519Signature),
        #[prost(message, tag="3")]
        MultiEd25519(super::MultiEd25519Signature),
    }
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MoveTypes {
    Bool = 0,
    U8 = 1,
    U64 = 2,
    U128 = 3,
    Address = 4,
    Signer = 5,
    /// { items: Box<MoveType> },
    Vector = 6,
    ///(MoveStructTag),
    Struct = 7,
    /// { index: u16 },
    GenericTypeParam = 8,
    /// { mutable: bool, to: Box<MoveType> },
    Reference = 9,
    ///(String),
    Unparsable = 10,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MoveAbility {
    Copy = 0,
    Drop = 1,
    Store = 2,
    Key = 3,
}

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_TRANSACTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.Transaction")] impl :: prost_wkt :: MessageSerde for Transaction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "Transaction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.Transaction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_TRANSACTION_TRIMMED : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.TransactionTrimmed")] impl :: prost_wkt :: MessageSerde for TransactionTrimmed { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "TransactionTrimmed" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.TransactionTrimmed" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_BLOCK_METADATA_TRANSACTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.BlockMetadataTransaction")] impl :: prost_wkt :: MessageSerde for BlockMetadataTransaction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "BlockMetadataTransaction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.BlockMetadataTransaction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_GENESIS_TRANSACTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.GenesisTransaction")] impl :: prost_wkt :: MessageSerde for GenesisTransaction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "GenesisTransaction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.GenesisTransaction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_STATE_CHECKPOINT_TRANSACTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.StateCheckpointTransaction")] impl :: prost_wkt :: MessageSerde for StateCheckpointTransaction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "StateCheckpointTransaction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.StateCheckpointTransaction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_USER_TRANSACTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.UserTransaction")] impl :: prost_wkt :: MessageSerde for UserTransaction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "UserTransaction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.UserTransaction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_EVENT : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.Event")] impl :: prost_wkt :: MessageSerde for Event { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "Event" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.Event" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_TRANSACTION_INFO : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.TransactionInfo")] impl :: prost_wkt :: MessageSerde for TransactionInfo { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "TransactionInfo" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.TransactionInfo" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_EVENT_KEY : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.EventKey")] impl :: prost_wkt :: MessageSerde for EventKey { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "EventKey" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.EventKey" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_USER_TRANSACTION_REQUEST : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.UserTransactionRequest")] impl :: prost_wkt :: MessageSerde for UserTransactionRequest { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "UserTransactionRequest" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.UserTransactionRequest" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_SET : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteSet")] impl :: prost_wkt :: MessageSerde for WriteSet { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteSet" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteSet" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_SCRIPT_WRITE_SET : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.ScriptWriteSet")] impl :: prost_wkt :: MessageSerde for ScriptWriteSet { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "ScriptWriteSet" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.ScriptWriteSet" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_DIRECT_WRITE_SET : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.DirectWriteSet")] impl :: prost_wkt :: MessageSerde for DirectWriteSet { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "DirectWriteSet" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.DirectWriteSet" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_SET_CHANGE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteSetChange")] impl :: prost_wkt :: MessageSerde for WriteSetChange { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteSetChange" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteSetChange" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_DELETE_MODULE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.DeleteModule")] impl :: prost_wkt :: MessageSerde for DeleteModule { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "DeleteModule" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.DeleteModule" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_DELETE_RESOURCE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.DeleteResource")] impl :: prost_wkt :: MessageSerde for DeleteResource { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "DeleteResource" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.DeleteResource" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_DELETE_TABLE_ITEM : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.DeleteTableItem")] impl :: prost_wkt :: MessageSerde for DeleteTableItem { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "DeleteTableItem" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.DeleteTableItem" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_DELETE_TABLE_DATA : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.DeleteTableData")] impl :: prost_wkt :: MessageSerde for DeleteTableData { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "DeleteTableData" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.DeleteTableData" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_MODULE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteModule")] impl :: prost_wkt :: MessageSerde for WriteModule { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteModule" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteModule" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_RESOURCE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteResource")] impl :: prost_wkt :: MessageSerde for WriteResource { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteResource" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteResource" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_TABLE_DATA : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteTableData")] impl :: prost_wkt :: MessageSerde for WriteTableData { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteTableData" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteTableData" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_TABLE_ITEM : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteTableItem")] impl :: prost_wkt :: MessageSerde for WriteTableItem { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteTableItem" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteTableItem" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_TRANSACTION_PAYLOAD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.TransactionPayload")] impl :: prost_wkt :: MessageSerde for TransactionPayload { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "TransactionPayload" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.TransactionPayload" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_SCRIPT_FUNCTION_PAYLOAD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.ScriptFunctionPayload")] impl :: prost_wkt :: MessageSerde for ScriptFunctionPayload { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "ScriptFunctionPayload" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.ScriptFunctionPayload" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_SCRIPT_BYTECODE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveScriptBytecode")] impl :: prost_wkt :: MessageSerde for MoveScriptBytecode { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveScriptBytecode" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveScriptBytecode" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_SCRIPT_PAYLOAD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.ScriptPayload")] impl :: prost_wkt :: MessageSerde for ScriptPayload { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "ScriptPayload" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.ScriptPayload" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MODULE_BUNDLE_PAYLOAD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.ModuleBundlePayload")] impl :: prost_wkt :: MessageSerde for ModuleBundlePayload { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "ModuleBundlePayload" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.ModuleBundlePayload" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_MODULE_BYTECODE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveModuleBytecode")] impl :: prost_wkt :: MessageSerde for MoveModuleBytecode { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveModuleBytecode" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveModuleBytecode" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_MODULE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveModule")] impl :: prost_wkt :: MessageSerde for MoveModule { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveModule" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveModule" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_FUNCTION : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveFunction")] impl :: prost_wkt :: MessageSerde for MoveFunction { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveFunction" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveFunction" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_STRUCT : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveStruct")] impl :: prost_wkt :: MessageSerde for MoveStruct { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveStruct" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveStruct" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_STRUCT_GENERIC_TYPE_PARAM : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveStructGenericTypeParam")] impl :: prost_wkt :: MessageSerde for MoveStructGenericTypeParam { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveStructGenericTypeParam" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveStructGenericTypeParam" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_STRUCT_FIELD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveStructField")] impl :: prost_wkt :: MessageSerde for MoveStructField { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveStructField" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveStructField" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_FUNCTION_GENERIC_TYPE_PARAM : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveFunctionGenericTypeParam")] impl :: prost_wkt :: MessageSerde for MoveFunctionGenericTypeParam { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveFunctionGenericTypeParam" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveFunctionGenericTypeParam" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_TYPE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveType")] impl :: prost_wkt :: MessageSerde for MoveType { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveType" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveType" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_WRITE_SET_PAYLOAD : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.WriteSetPayload")] impl :: prost_wkt :: MessageSerde for WriteSetPayload { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "WriteSetPayload" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.WriteSetPayload" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_SCRIPT_FUNCTION_ID : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.ScriptFunctionId")] impl :: prost_wkt :: MessageSerde for ScriptFunctionId { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "ScriptFunctionId" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.ScriptFunctionId" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_RESOURCE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveResource")] impl :: prost_wkt :: MessageSerde for MoveResource { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveResource" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveResource" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_MODULE_ID : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveModuleId")] impl :: prost_wkt :: MessageSerde for MoveModuleId { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveModuleId" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveModuleId" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MOVE_STRUCT_TAG : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MoveStructTag")] impl :: prost_wkt :: MessageSerde for MoveStructTag { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MoveStructTag" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MoveStructTag" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_SIGNATURE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.Signature")] impl :: prost_wkt :: MessageSerde for Signature { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "Signature" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.Signature" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_ED_25519_SIGNATURE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.Ed25519Signature")] impl :: prost_wkt :: MessageSerde for Ed25519Signature { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "Ed25519Signature" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.Ed25519Signature" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MULTI_ED_25519_SIGNATURE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MultiEd25519Signature")] impl :: prost_wkt :: MessageSerde for MultiEd25519Signature { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MultiEd25519Signature" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MultiEd25519Signature" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_MULTI_AGENT_SIGNATURE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.MultiAgentSignature")] impl :: prost_wkt :: MessageSerde for MultiAgentSignature { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "MultiAgentSignature" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.MultiAgentSignature" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;

# [allow (dead_code)] const IMPL_MESSAGE_SERDE_FOR_ACCOUNT_SIGNATURE : () = { use :: prost_wkt :: typetag ; # [typetag :: serde (name = "type.googleapis.com/aptos.extractor.v1.AccountSignature")] impl :: prost_wkt :: MessageSerde for AccountSignature { fn package_name (& self) -> & 'static str { "aptos.extractor.v1" } fn message_name (& self) -> & 'static str { "AccountSignature" } fn type_url (& self) -> & 'static str { "type.googleapis.com/aptos.extractor.v1.AccountSignature" } fn new_instance (& self , data : Vec < u8 >) -> Result < Box < dyn :: prost_wkt :: MessageSerde > , :: prost :: DecodeError > { let mut target = Self :: default () ; :: prost :: Message :: merge (& mut target , data . as_slice ()) ? ; let erased : Box < dyn :: prost_wkt :: MessageSerde > = Box :: new (target) ; Ok (erased) } fn encoded (& self) -> Vec < u8 > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) . expect ("Failed to encode message") ; buf } fn try_encoded (& self) -> Result < Vec < u8 > , :: prost :: EncodeError > { let mut buf = Vec :: new () ; buf . reserve (:: prost :: Message :: encoded_len (self)) ; :: prost :: Message :: encode (self , & mut buf) ? ; Ok (buf) } } } ;
