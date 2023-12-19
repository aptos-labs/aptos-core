from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from aptos.util.timestamp import timestamp_pb2 as _timestamp_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

DESCRIPTOR: _descriptor.FileDescriptor

class MoveTypes(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    MOVE_TYPES_UNSPECIFIED: _ClassVar[MoveTypes]
    MOVE_TYPES_BOOL: _ClassVar[MoveTypes]
    MOVE_TYPES_U8: _ClassVar[MoveTypes]
    MOVE_TYPES_U16: _ClassVar[MoveTypes]
    MOVE_TYPES_U32: _ClassVar[MoveTypes]
    MOVE_TYPES_U64: _ClassVar[MoveTypes]
    MOVE_TYPES_U128: _ClassVar[MoveTypes]
    MOVE_TYPES_U256: _ClassVar[MoveTypes]
    MOVE_TYPES_ADDRESS: _ClassVar[MoveTypes]
    MOVE_TYPES_SIGNER: _ClassVar[MoveTypes]
    MOVE_TYPES_VECTOR: _ClassVar[MoveTypes]
    MOVE_TYPES_STRUCT: _ClassVar[MoveTypes]
    MOVE_TYPES_GENERIC_TYPE_PARAM: _ClassVar[MoveTypes]
    MOVE_TYPES_REFERENCE: _ClassVar[MoveTypes]
    MOVE_TYPES_UNPARSABLE: _ClassVar[MoveTypes]

class MoveAbility(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    MOVE_ABILITY_UNSPECIFIED: _ClassVar[MoveAbility]
    MOVE_ABILITY_COPY: _ClassVar[MoveAbility]
    MOVE_ABILITY_DROP: _ClassVar[MoveAbility]
    MOVE_ABILITY_STORE: _ClassVar[MoveAbility]
    MOVE_ABILITY_KEY: _ClassVar[MoveAbility]

MOVE_TYPES_UNSPECIFIED: MoveTypes
MOVE_TYPES_BOOL: MoveTypes
MOVE_TYPES_U8: MoveTypes
MOVE_TYPES_U16: MoveTypes
MOVE_TYPES_U32: MoveTypes
MOVE_TYPES_U64: MoveTypes
MOVE_TYPES_U128: MoveTypes
MOVE_TYPES_U256: MoveTypes
MOVE_TYPES_ADDRESS: MoveTypes
MOVE_TYPES_SIGNER: MoveTypes
MOVE_TYPES_VECTOR: MoveTypes
MOVE_TYPES_STRUCT: MoveTypes
MOVE_TYPES_GENERIC_TYPE_PARAM: MoveTypes
MOVE_TYPES_REFERENCE: MoveTypes
MOVE_TYPES_UNPARSABLE: MoveTypes
MOVE_ABILITY_UNSPECIFIED: MoveAbility
MOVE_ABILITY_COPY: MoveAbility
MOVE_ABILITY_DROP: MoveAbility
MOVE_ABILITY_STORE: MoveAbility
MOVE_ABILITY_KEY: MoveAbility

class Block(_message.Message):
    __slots__ = ["timestamp", "height", "transactions", "chain_id"]
    TIMESTAMP_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    TRANSACTIONS_FIELD_NUMBER: _ClassVar[int]
    CHAIN_ID_FIELD_NUMBER: _ClassVar[int]
    timestamp: _timestamp_pb2.Timestamp
    height: int
    transactions: _containers.RepeatedCompositeFieldContainer[Transaction]
    chain_id: int
    def __init__(
        self,
        timestamp: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        height: _Optional[int] = ...,
        transactions: _Optional[_Iterable[_Union[Transaction, _Mapping]]] = ...,
        chain_id: _Optional[int] = ...,
    ) -> None: ...

class Transaction(_message.Message):
    __slots__ = [
        "timestamp",
        "version",
        "info",
        "epoch",
        "block_height",
        "type",
        "block_metadata",
        "genesis",
        "state_checkpoint",
        "user",
        "validator",
    ]

    class TransactionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TRANSACTION_TYPE_UNSPECIFIED: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_GENESIS: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_BLOCK_METADATA: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_STATE_CHECKPOINT: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_USER: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_VALIDATOR: _ClassVar[Transaction.TransactionType]
    TRANSACTION_TYPE_UNSPECIFIED: Transaction.TransactionType
    TRANSACTION_TYPE_GENESIS: Transaction.TransactionType
    TRANSACTION_TYPE_BLOCK_METADATA: Transaction.TransactionType
    TRANSACTION_TYPE_STATE_CHECKPOINT: Transaction.TransactionType
    TRANSACTION_TYPE_USER: Transaction.TransactionType
    TRANSACTION_TYPE_VALIDATOR: Transaction.TransactionType
    TIMESTAMP_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    INFO_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    BLOCK_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_METADATA_FIELD_NUMBER: _ClassVar[int]
    GENESIS_FIELD_NUMBER: _ClassVar[int]
    STATE_CHECKPOINT_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    VALIDATOR_FIELD_NUMBER: _ClassVar[int]
    timestamp: _timestamp_pb2.Timestamp
    version: int
    info: TransactionInfo
    epoch: int
    block_height: int
    type: Transaction.TransactionType
    block_metadata: BlockMetadataTransaction
    genesis: GenesisTransaction
    state_checkpoint: StateCheckpointTransaction
    user: UserTransaction
    validator: ValidatorTransaction
    def __init__(
        self,
        timestamp: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        version: _Optional[int] = ...,
        info: _Optional[_Union[TransactionInfo, _Mapping]] = ...,
        epoch: _Optional[int] = ...,
        block_height: _Optional[int] = ...,
        type: _Optional[_Union[Transaction.TransactionType, str]] = ...,
        block_metadata: _Optional[_Union[BlockMetadataTransaction, _Mapping]] = ...,
        genesis: _Optional[_Union[GenesisTransaction, _Mapping]] = ...,
        state_checkpoint: _Optional[_Union[StateCheckpointTransaction, _Mapping]] = ...,
        user: _Optional[_Union[UserTransaction, _Mapping]] = ...,
        validator: _Optional[_Union[ValidatorTransaction, _Mapping]] = ...,
    ) -> None: ...

class BlockMetadataTransaction(_message.Message):
    __slots__ = [
        "id",
        "round",
        "events",
        "previous_block_votes_bitvec",
        "proposer",
        "failed_proposer_indices",
    ]
    ID_FIELD_NUMBER: _ClassVar[int]
    ROUND_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    PREVIOUS_BLOCK_VOTES_BITVEC_FIELD_NUMBER: _ClassVar[int]
    PROPOSER_FIELD_NUMBER: _ClassVar[int]
    FAILED_PROPOSER_INDICES_FIELD_NUMBER: _ClassVar[int]
    id: str
    round: int
    events: _containers.RepeatedCompositeFieldContainer[Event]
    previous_block_votes_bitvec: bytes
    proposer: str
    failed_proposer_indices: _containers.RepeatedScalarFieldContainer[int]
    def __init__(
        self,
        id: _Optional[str] = ...,
        round: _Optional[int] = ...,
        events: _Optional[_Iterable[_Union[Event, _Mapping]]] = ...,
        previous_block_votes_bitvec: _Optional[bytes] = ...,
        proposer: _Optional[str] = ...,
        failed_proposer_indices: _Optional[_Iterable[int]] = ...,
    ) -> None: ...

class GenesisTransaction(_message.Message):
    __slots__ = ["payload", "events"]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    payload: WriteSet
    events: _containers.RepeatedCompositeFieldContainer[Event]
    def __init__(
        self,
        payload: _Optional[_Union[WriteSet, _Mapping]] = ...,
        events: _Optional[_Iterable[_Union[Event, _Mapping]]] = ...,
    ) -> None: ...

class StateCheckpointTransaction(_message.Message):
    __slots__ = []
    def __init__(self) -> None: ...

class ValidatorTransaction(_message.Message):
    __slots__ = []
    def __init__(self) -> None: ...

class UserTransaction(_message.Message):
    __slots__ = ["request", "events"]
    REQUEST_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    request: UserTransactionRequest
    events: _containers.RepeatedCompositeFieldContainer[Event]
    def __init__(
        self,
        request: _Optional[_Union[UserTransactionRequest, _Mapping]] = ...,
        events: _Optional[_Iterable[_Union[Event, _Mapping]]] = ...,
    ) -> None: ...

class Event(_message.Message):
    __slots__ = ["key", "sequence_number", "type", "type_str", "data"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    SEQUENCE_NUMBER_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_STR_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    key: EventKey
    sequence_number: int
    type: MoveType
    type_str: str
    data: str
    def __init__(
        self,
        key: _Optional[_Union[EventKey, _Mapping]] = ...,
        sequence_number: _Optional[int] = ...,
        type: _Optional[_Union[MoveType, _Mapping]] = ...,
        type_str: _Optional[str] = ...,
        data: _Optional[str] = ...,
    ) -> None: ...

class TransactionInfo(_message.Message):
    __slots__ = [
        "hash",
        "state_change_hash",
        "event_root_hash",
        "state_checkpoint_hash",
        "gas_used",
        "success",
        "vm_status",
        "accumulator_root_hash",
        "changes",
    ]
    HASH_FIELD_NUMBER: _ClassVar[int]
    STATE_CHANGE_HASH_FIELD_NUMBER: _ClassVar[int]
    EVENT_ROOT_HASH_FIELD_NUMBER: _ClassVar[int]
    STATE_CHECKPOINT_HASH_FIELD_NUMBER: _ClassVar[int]
    GAS_USED_FIELD_NUMBER: _ClassVar[int]
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    VM_STATUS_FIELD_NUMBER: _ClassVar[int]
    ACCUMULATOR_ROOT_HASH_FIELD_NUMBER: _ClassVar[int]
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    hash: bytes
    state_change_hash: bytes
    event_root_hash: bytes
    state_checkpoint_hash: bytes
    gas_used: int
    success: bool
    vm_status: str
    accumulator_root_hash: bytes
    changes: _containers.RepeatedCompositeFieldContainer[WriteSetChange]
    def __init__(
        self,
        hash: _Optional[bytes] = ...,
        state_change_hash: _Optional[bytes] = ...,
        event_root_hash: _Optional[bytes] = ...,
        state_checkpoint_hash: _Optional[bytes] = ...,
        gas_used: _Optional[int] = ...,
        success: bool = ...,
        vm_status: _Optional[str] = ...,
        accumulator_root_hash: _Optional[bytes] = ...,
        changes: _Optional[_Iterable[_Union[WriteSetChange, _Mapping]]] = ...,
    ) -> None: ...

class EventKey(_message.Message):
    __slots__ = ["creation_number", "account_address"]
    CREATION_NUMBER_FIELD_NUMBER: _ClassVar[int]
    ACCOUNT_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    creation_number: int
    account_address: str
    def __init__(
        self,
        creation_number: _Optional[int] = ...,
        account_address: _Optional[str] = ...,
    ) -> None: ...

class UserTransactionRequest(_message.Message):
    __slots__ = [
        "sender",
        "sequence_number",
        "max_gas_amount",
        "gas_unit_price",
        "expiration_timestamp_secs",
        "payload",
        "signature",
    ]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    SEQUENCE_NUMBER_FIELD_NUMBER: _ClassVar[int]
    MAX_GAS_AMOUNT_FIELD_NUMBER: _ClassVar[int]
    GAS_UNIT_PRICE_FIELD_NUMBER: _ClassVar[int]
    EXPIRATION_TIMESTAMP_SECS_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    sender: str
    sequence_number: int
    max_gas_amount: int
    gas_unit_price: int
    expiration_timestamp_secs: _timestamp_pb2.Timestamp
    payload: TransactionPayload
    signature: Signature
    def __init__(
        self,
        sender: _Optional[str] = ...,
        sequence_number: _Optional[int] = ...,
        max_gas_amount: _Optional[int] = ...,
        gas_unit_price: _Optional[int] = ...,
        expiration_timestamp_secs: _Optional[
            _Union[_timestamp_pb2.Timestamp, _Mapping]
        ] = ...,
        payload: _Optional[_Union[TransactionPayload, _Mapping]] = ...,
        signature: _Optional[_Union[Signature, _Mapping]] = ...,
    ) -> None: ...

class WriteSet(_message.Message):
    __slots__ = ["write_set_type", "script_write_set", "direct_write_set"]

    class WriteSetType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        WRITE_SET_TYPE_UNSPECIFIED: _ClassVar[WriteSet.WriteSetType]
        WRITE_SET_TYPE_SCRIPT_WRITE_SET: _ClassVar[WriteSet.WriteSetType]
        WRITE_SET_TYPE_DIRECT_WRITE_SET: _ClassVar[WriteSet.WriteSetType]
    WRITE_SET_TYPE_UNSPECIFIED: WriteSet.WriteSetType
    WRITE_SET_TYPE_SCRIPT_WRITE_SET: WriteSet.WriteSetType
    WRITE_SET_TYPE_DIRECT_WRITE_SET: WriteSet.WriteSetType
    WRITE_SET_TYPE_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_WRITE_SET_FIELD_NUMBER: _ClassVar[int]
    DIRECT_WRITE_SET_FIELD_NUMBER: _ClassVar[int]
    write_set_type: WriteSet.WriteSetType
    script_write_set: ScriptWriteSet
    direct_write_set: DirectWriteSet
    def __init__(
        self,
        write_set_type: _Optional[_Union[WriteSet.WriteSetType, str]] = ...,
        script_write_set: _Optional[_Union[ScriptWriteSet, _Mapping]] = ...,
        direct_write_set: _Optional[_Union[DirectWriteSet, _Mapping]] = ...,
    ) -> None: ...

class ScriptWriteSet(_message.Message):
    __slots__ = ["execute_as", "script"]
    EXECUTE_AS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_FIELD_NUMBER: _ClassVar[int]
    execute_as: str
    script: ScriptPayload
    def __init__(
        self,
        execute_as: _Optional[str] = ...,
        script: _Optional[_Union[ScriptPayload, _Mapping]] = ...,
    ) -> None: ...

class DirectWriteSet(_message.Message):
    __slots__ = ["write_set_change", "events"]
    WRITE_SET_CHANGE_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    write_set_change: _containers.RepeatedCompositeFieldContainer[WriteSetChange]
    events: _containers.RepeatedCompositeFieldContainer[Event]
    def __init__(
        self,
        write_set_change: _Optional[_Iterable[_Union[WriteSetChange, _Mapping]]] = ...,
        events: _Optional[_Iterable[_Union[Event, _Mapping]]] = ...,
    ) -> None: ...

class WriteSetChange(_message.Message):
    __slots__ = [
        "type",
        "delete_module",
        "delete_resource",
        "delete_table_item",
        "write_module",
        "write_resource",
        "write_table_item",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[WriteSetChange.Type]
        TYPE_DELETE_MODULE: _ClassVar[WriteSetChange.Type]
        TYPE_DELETE_RESOURCE: _ClassVar[WriteSetChange.Type]
        TYPE_DELETE_TABLE_ITEM: _ClassVar[WriteSetChange.Type]
        TYPE_WRITE_MODULE: _ClassVar[WriteSetChange.Type]
        TYPE_WRITE_RESOURCE: _ClassVar[WriteSetChange.Type]
        TYPE_WRITE_TABLE_ITEM: _ClassVar[WriteSetChange.Type]
    TYPE_UNSPECIFIED: WriteSetChange.Type
    TYPE_DELETE_MODULE: WriteSetChange.Type
    TYPE_DELETE_RESOURCE: WriteSetChange.Type
    TYPE_DELETE_TABLE_ITEM: WriteSetChange.Type
    TYPE_WRITE_MODULE: WriteSetChange.Type
    TYPE_WRITE_RESOURCE: WriteSetChange.Type
    TYPE_WRITE_TABLE_ITEM: WriteSetChange.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    DELETE_MODULE_FIELD_NUMBER: _ClassVar[int]
    DELETE_RESOURCE_FIELD_NUMBER: _ClassVar[int]
    DELETE_TABLE_ITEM_FIELD_NUMBER: _ClassVar[int]
    WRITE_MODULE_FIELD_NUMBER: _ClassVar[int]
    WRITE_RESOURCE_FIELD_NUMBER: _ClassVar[int]
    WRITE_TABLE_ITEM_FIELD_NUMBER: _ClassVar[int]
    type: WriteSetChange.Type
    delete_module: DeleteModule
    delete_resource: DeleteResource
    delete_table_item: DeleteTableItem
    write_module: WriteModule
    write_resource: WriteResource
    write_table_item: WriteTableItem
    def __init__(
        self,
        type: _Optional[_Union[WriteSetChange.Type, str]] = ...,
        delete_module: _Optional[_Union[DeleteModule, _Mapping]] = ...,
        delete_resource: _Optional[_Union[DeleteResource, _Mapping]] = ...,
        delete_table_item: _Optional[_Union[DeleteTableItem, _Mapping]] = ...,
        write_module: _Optional[_Union[WriteModule, _Mapping]] = ...,
        write_resource: _Optional[_Union[WriteResource, _Mapping]] = ...,
        write_table_item: _Optional[_Union[WriteTableItem, _Mapping]] = ...,
    ) -> None: ...

class DeleteModule(_message.Message):
    __slots__ = ["address", "state_key_hash", "module"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    MODULE_FIELD_NUMBER: _ClassVar[int]
    address: str
    state_key_hash: bytes
    module: MoveModuleId
    def __init__(
        self,
        address: _Optional[str] = ...,
        state_key_hash: _Optional[bytes] = ...,
        module: _Optional[_Union[MoveModuleId, _Mapping]] = ...,
    ) -> None: ...

class DeleteResource(_message.Message):
    __slots__ = ["address", "state_key_hash", "type", "type_str"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_STR_FIELD_NUMBER: _ClassVar[int]
    address: str
    state_key_hash: bytes
    type: MoveStructTag
    type_str: str
    def __init__(
        self,
        address: _Optional[str] = ...,
        state_key_hash: _Optional[bytes] = ...,
        type: _Optional[_Union[MoveStructTag, _Mapping]] = ...,
        type_str: _Optional[str] = ...,
    ) -> None: ...

class DeleteTableItem(_message.Message):
    __slots__ = ["state_key_hash", "handle", "key", "data"]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    state_key_hash: bytes
    handle: str
    key: str
    data: DeleteTableData
    def __init__(
        self,
        state_key_hash: _Optional[bytes] = ...,
        handle: _Optional[str] = ...,
        key: _Optional[str] = ...,
        data: _Optional[_Union[DeleteTableData, _Mapping]] = ...,
    ) -> None: ...

class DeleteTableData(_message.Message):
    __slots__ = ["key", "key_type"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    key: str
    key_type: str
    def __init__(
        self, key: _Optional[str] = ..., key_type: _Optional[str] = ...
    ) -> None: ...

class WriteModule(_message.Message):
    __slots__ = ["address", "state_key_hash", "data"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    state_key_hash: bytes
    data: MoveModuleBytecode
    def __init__(
        self,
        address: _Optional[str] = ...,
        state_key_hash: _Optional[bytes] = ...,
        data: _Optional[_Union[MoveModuleBytecode, _Mapping]] = ...,
    ) -> None: ...

class WriteResource(_message.Message):
    __slots__ = ["address", "state_key_hash", "type", "type_str", "data"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_STR_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    state_key_hash: bytes
    type: MoveStructTag
    type_str: str
    data: str
    def __init__(
        self,
        address: _Optional[str] = ...,
        state_key_hash: _Optional[bytes] = ...,
        type: _Optional[_Union[MoveStructTag, _Mapping]] = ...,
        type_str: _Optional[str] = ...,
        data: _Optional[str] = ...,
    ) -> None: ...

class WriteTableData(_message.Message):
    __slots__ = ["key", "key_type", "value", "value_type"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    key: str
    key_type: str
    value: str
    value_type: str
    def __init__(
        self,
        key: _Optional[str] = ...,
        key_type: _Optional[str] = ...,
        value: _Optional[str] = ...,
        value_type: _Optional[str] = ...,
    ) -> None: ...

class WriteTableItem(_message.Message):
    __slots__ = ["state_key_hash", "handle", "key", "data"]
    STATE_KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    state_key_hash: bytes
    handle: str
    key: str
    data: WriteTableData
    def __init__(
        self,
        state_key_hash: _Optional[bytes] = ...,
        handle: _Optional[str] = ...,
        key: _Optional[str] = ...,
        data: _Optional[_Union[WriteTableData, _Mapping]] = ...,
    ) -> None: ...

class TransactionPayload(_message.Message):
    __slots__ = [
        "type",
        "entry_function_payload",
        "script_payload",
        "module_bundle_payload",
        "write_set_payload",
        "multisig_payload",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[TransactionPayload.Type]
        TYPE_ENTRY_FUNCTION_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_SCRIPT_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_MODULE_BUNDLE_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_WRITE_SET_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_MULTISIG_PAYLOAD: _ClassVar[TransactionPayload.Type]
    TYPE_UNSPECIFIED: TransactionPayload.Type
    TYPE_ENTRY_FUNCTION_PAYLOAD: TransactionPayload.Type
    TYPE_SCRIPT_PAYLOAD: TransactionPayload.Type
    TYPE_MODULE_BUNDLE_PAYLOAD: TransactionPayload.Type
    TYPE_WRITE_SET_PAYLOAD: TransactionPayload.Type
    TYPE_MULTISIG_PAYLOAD: TransactionPayload.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ENTRY_FUNCTION_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    MODULE_BUNDLE_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    WRITE_SET_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    MULTISIG_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    type: TransactionPayload.Type
    entry_function_payload: EntryFunctionPayload
    script_payload: ScriptPayload
    module_bundle_payload: ModuleBundlePayload
    write_set_payload: WriteSetPayload
    multisig_payload: MultisigPayload
    def __init__(
        self,
        type: _Optional[_Union[TransactionPayload.Type, str]] = ...,
        entry_function_payload: _Optional[_Union[EntryFunctionPayload, _Mapping]] = ...,
        script_payload: _Optional[_Union[ScriptPayload, _Mapping]] = ...,
        module_bundle_payload: _Optional[_Union[ModuleBundlePayload, _Mapping]] = ...,
        write_set_payload: _Optional[_Union[WriteSetPayload, _Mapping]] = ...,
        multisig_payload: _Optional[_Union[MultisigPayload, _Mapping]] = ...,
    ) -> None: ...

class EntryFunctionPayload(_message.Message):
    __slots__ = ["function", "type_arguments", "arguments", "entry_function_id_str"]
    FUNCTION_FIELD_NUMBER: _ClassVar[int]
    TYPE_ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    ENTRY_FUNCTION_ID_STR_FIELD_NUMBER: _ClassVar[int]
    function: EntryFunctionId
    type_arguments: _containers.RepeatedCompositeFieldContainer[MoveType]
    arguments: _containers.RepeatedScalarFieldContainer[str]
    entry_function_id_str: str
    def __init__(
        self,
        function: _Optional[_Union[EntryFunctionId, _Mapping]] = ...,
        type_arguments: _Optional[_Iterable[_Union[MoveType, _Mapping]]] = ...,
        arguments: _Optional[_Iterable[str]] = ...,
        entry_function_id_str: _Optional[str] = ...,
    ) -> None: ...

class MoveScriptBytecode(_message.Message):
    __slots__ = ["bytecode", "abi"]
    BYTECODE_FIELD_NUMBER: _ClassVar[int]
    ABI_FIELD_NUMBER: _ClassVar[int]
    bytecode: bytes
    abi: MoveFunction
    def __init__(
        self,
        bytecode: _Optional[bytes] = ...,
        abi: _Optional[_Union[MoveFunction, _Mapping]] = ...,
    ) -> None: ...

class ScriptPayload(_message.Message):
    __slots__ = ["code", "type_arguments", "arguments"]
    CODE_FIELD_NUMBER: _ClassVar[int]
    TYPE_ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    code: MoveScriptBytecode
    type_arguments: _containers.RepeatedCompositeFieldContainer[MoveType]
    arguments: _containers.RepeatedScalarFieldContainer[str]
    def __init__(
        self,
        code: _Optional[_Union[MoveScriptBytecode, _Mapping]] = ...,
        type_arguments: _Optional[_Iterable[_Union[MoveType, _Mapping]]] = ...,
        arguments: _Optional[_Iterable[str]] = ...,
    ) -> None: ...

class MultisigPayload(_message.Message):
    __slots__ = ["multisig_address", "transaction_payload"]
    MULTISIG_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    TRANSACTION_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    multisig_address: str
    transaction_payload: MultisigTransactionPayload
    def __init__(
        self,
        multisig_address: _Optional[str] = ...,
        transaction_payload: _Optional[
            _Union[MultisigTransactionPayload, _Mapping]
        ] = ...,
    ) -> None: ...

class MultisigTransactionPayload(_message.Message):
    __slots__ = ["type", "entry_function_payload"]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[MultisigTransactionPayload.Type]
        TYPE_ENTRY_FUNCTION_PAYLOAD: _ClassVar[MultisigTransactionPayload.Type]
    TYPE_UNSPECIFIED: MultisigTransactionPayload.Type
    TYPE_ENTRY_FUNCTION_PAYLOAD: MultisigTransactionPayload.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ENTRY_FUNCTION_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    type: MultisigTransactionPayload.Type
    entry_function_payload: EntryFunctionPayload
    def __init__(
        self,
        type: _Optional[_Union[MultisigTransactionPayload.Type, str]] = ...,
        entry_function_payload: _Optional[_Union[EntryFunctionPayload, _Mapping]] = ...,
    ) -> None: ...

class ModuleBundlePayload(_message.Message):
    __slots__ = ["modules"]
    MODULES_FIELD_NUMBER: _ClassVar[int]
    modules: _containers.RepeatedCompositeFieldContainer[MoveModuleBytecode]
    def __init__(
        self, modules: _Optional[_Iterable[_Union[MoveModuleBytecode, _Mapping]]] = ...
    ) -> None: ...

class MoveModuleBytecode(_message.Message):
    __slots__ = ["bytecode", "abi"]
    BYTECODE_FIELD_NUMBER: _ClassVar[int]
    ABI_FIELD_NUMBER: _ClassVar[int]
    bytecode: bytes
    abi: MoveModule
    def __init__(
        self,
        bytecode: _Optional[bytes] = ...,
        abi: _Optional[_Union[MoveModule, _Mapping]] = ...,
    ) -> None: ...

class MoveModule(_message.Message):
    __slots__ = ["address", "name", "friends", "exposed_functions", "structs"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    FRIENDS_FIELD_NUMBER: _ClassVar[int]
    EXPOSED_FUNCTIONS_FIELD_NUMBER: _ClassVar[int]
    STRUCTS_FIELD_NUMBER: _ClassVar[int]
    address: str
    name: str
    friends: _containers.RepeatedCompositeFieldContainer[MoveModuleId]
    exposed_functions: _containers.RepeatedCompositeFieldContainer[MoveFunction]
    structs: _containers.RepeatedCompositeFieldContainer[MoveStruct]
    def __init__(
        self,
        address: _Optional[str] = ...,
        name: _Optional[str] = ...,
        friends: _Optional[_Iterable[_Union[MoveModuleId, _Mapping]]] = ...,
        exposed_functions: _Optional[_Iterable[_Union[MoveFunction, _Mapping]]] = ...,
        structs: _Optional[_Iterable[_Union[MoveStruct, _Mapping]]] = ...,
    ) -> None: ...

class MoveFunction(_message.Message):
    __slots__ = ["name", "visibility", "is_entry", "generic_type_params", "params"]

    class Visibility(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        VISIBILITY_UNSPECIFIED: _ClassVar[MoveFunction.Visibility]
        VISIBILITY_PRIVATE: _ClassVar[MoveFunction.Visibility]
        VISIBILITY_PUBLIC: _ClassVar[MoveFunction.Visibility]
        VISIBILITY_FRIEND: _ClassVar[MoveFunction.Visibility]
    VISIBILITY_UNSPECIFIED: MoveFunction.Visibility
    VISIBILITY_PRIVATE: MoveFunction.Visibility
    VISIBILITY_PUBLIC: MoveFunction.Visibility
    VISIBILITY_FRIEND: MoveFunction.Visibility
    NAME_FIELD_NUMBER: _ClassVar[int]
    VISIBILITY_FIELD_NUMBER: _ClassVar[int]
    IS_ENTRY_FIELD_NUMBER: _ClassVar[int]
    GENERIC_TYPE_PARAMS_FIELD_NUMBER: _ClassVar[int]
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    RETURN_FIELD_NUMBER: _ClassVar[int]
    name: str
    visibility: MoveFunction.Visibility
    is_entry: bool
    generic_type_params: _containers.RepeatedCompositeFieldContainer[
        MoveFunctionGenericTypeParam
    ]
    params: _containers.RepeatedCompositeFieldContainer[MoveType]
    def __init__(
        self,
        name: _Optional[str] = ...,
        visibility: _Optional[_Union[MoveFunction.Visibility, str]] = ...,
        is_entry: bool = ...,
        generic_type_params: _Optional[
            _Iterable[_Union[MoveFunctionGenericTypeParam, _Mapping]]
        ] = ...,
        params: _Optional[_Iterable[_Union[MoveType, _Mapping]]] = ...,
        **kwargs
    ) -> None: ...

class MoveStruct(_message.Message):
    __slots__ = ["name", "is_native", "abilities", "generic_type_params", "fields"]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IS_NATIVE_FIELD_NUMBER: _ClassVar[int]
    ABILITIES_FIELD_NUMBER: _ClassVar[int]
    GENERIC_TYPE_PARAMS_FIELD_NUMBER: _ClassVar[int]
    FIELDS_FIELD_NUMBER: _ClassVar[int]
    name: str
    is_native: bool
    abilities: _containers.RepeatedScalarFieldContainer[MoveAbility]
    generic_type_params: _containers.RepeatedCompositeFieldContainer[
        MoveStructGenericTypeParam
    ]
    fields: _containers.RepeatedCompositeFieldContainer[MoveStructField]
    def __init__(
        self,
        name: _Optional[str] = ...,
        is_native: bool = ...,
        abilities: _Optional[_Iterable[_Union[MoveAbility, str]]] = ...,
        generic_type_params: _Optional[
            _Iterable[_Union[MoveStructGenericTypeParam, _Mapping]]
        ] = ...,
        fields: _Optional[_Iterable[_Union[MoveStructField, _Mapping]]] = ...,
    ) -> None: ...

class MoveStructGenericTypeParam(_message.Message):
    __slots__ = ["constraints", "is_phantom"]
    CONSTRAINTS_FIELD_NUMBER: _ClassVar[int]
    IS_PHANTOM_FIELD_NUMBER: _ClassVar[int]
    constraints: _containers.RepeatedScalarFieldContainer[MoveAbility]
    is_phantom: bool
    def __init__(
        self,
        constraints: _Optional[_Iterable[_Union[MoveAbility, str]]] = ...,
        is_phantom: bool = ...,
    ) -> None: ...

class MoveStructField(_message.Message):
    __slots__ = ["name", "type"]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    name: str
    type: MoveType
    def __init__(
        self,
        name: _Optional[str] = ...,
        type: _Optional[_Union[MoveType, _Mapping]] = ...,
    ) -> None: ...

class MoveFunctionGenericTypeParam(_message.Message):
    __slots__ = ["constraints"]
    CONSTRAINTS_FIELD_NUMBER: _ClassVar[int]
    constraints: _containers.RepeatedScalarFieldContainer[MoveAbility]
    def __init__(
        self, constraints: _Optional[_Iterable[_Union[MoveAbility, str]]] = ...
    ) -> None: ...

class MoveType(_message.Message):
    __slots__ = [
        "type",
        "vector",
        "struct",
        "generic_type_param_index",
        "reference",
        "unparsable",
    ]

    class ReferenceType(_message.Message):
        __slots__ = ["mutable", "to"]
        MUTABLE_FIELD_NUMBER: _ClassVar[int]
        TO_FIELD_NUMBER: _ClassVar[int]
        mutable: bool
        to: MoveType
        def __init__(
            self, mutable: bool = ..., to: _Optional[_Union[MoveType, _Mapping]] = ...
        ) -> None: ...
    TYPE_FIELD_NUMBER: _ClassVar[int]
    VECTOR_FIELD_NUMBER: _ClassVar[int]
    STRUCT_FIELD_NUMBER: _ClassVar[int]
    GENERIC_TYPE_PARAM_INDEX_FIELD_NUMBER: _ClassVar[int]
    REFERENCE_FIELD_NUMBER: _ClassVar[int]
    UNPARSABLE_FIELD_NUMBER: _ClassVar[int]
    type: MoveTypes
    vector: MoveType
    struct: MoveStructTag
    generic_type_param_index: int
    reference: MoveType.ReferenceType
    unparsable: str
    def __init__(
        self,
        type: _Optional[_Union[MoveTypes, str]] = ...,
        vector: _Optional[_Union[MoveType, _Mapping]] = ...,
        struct: _Optional[_Union[MoveStructTag, _Mapping]] = ...,
        generic_type_param_index: _Optional[int] = ...,
        reference: _Optional[_Union[MoveType.ReferenceType, _Mapping]] = ...,
        unparsable: _Optional[str] = ...,
    ) -> None: ...

class WriteSetPayload(_message.Message):
    __slots__ = ["write_set"]
    WRITE_SET_FIELD_NUMBER: _ClassVar[int]
    write_set: WriteSet
    def __init__(
        self, write_set: _Optional[_Union[WriteSet, _Mapping]] = ...
    ) -> None: ...

class EntryFunctionId(_message.Message):
    __slots__ = ["module", "name"]
    MODULE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    module: MoveModuleId
    name: str
    def __init__(
        self,
        module: _Optional[_Union[MoveModuleId, _Mapping]] = ...,
        name: _Optional[str] = ...,
    ) -> None: ...

class MoveModuleId(_message.Message):
    __slots__ = ["address", "name"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    address: str
    name: str
    def __init__(
        self, address: _Optional[str] = ..., name: _Optional[str] = ...
    ) -> None: ...

class MoveStructTag(_message.Message):
    __slots__ = ["address", "module", "name", "generic_type_params"]
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    MODULE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    GENERIC_TYPE_PARAMS_FIELD_NUMBER: _ClassVar[int]
    address: str
    module: str
    name: str
    generic_type_params: _containers.RepeatedCompositeFieldContainer[MoveType]
    def __init__(
        self,
        address: _Optional[str] = ...,
        module: _Optional[str] = ...,
        name: _Optional[str] = ...,
        generic_type_params: _Optional[_Iterable[_Union[MoveType, _Mapping]]] = ...,
    ) -> None: ...

class Signature(_message.Message):
    __slots__ = [
        "type",
        "ed25519",
        "multi_ed25519",
        "multi_agent",
        "fee_payer",
        "single_sender",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[Signature.Type]
        TYPE_ED25519: _ClassVar[Signature.Type]
        TYPE_MULTI_ED25519: _ClassVar[Signature.Type]
        TYPE_MULTI_AGENT: _ClassVar[Signature.Type]
        TYPE_FEE_PAYER: _ClassVar[Signature.Type]
        TYPE_SINGLE_SENDER: _ClassVar[Signature.Type]
    TYPE_UNSPECIFIED: Signature.Type
    TYPE_ED25519: Signature.Type
    TYPE_MULTI_ED25519: Signature.Type
    TYPE_MULTI_AGENT: Signature.Type
    TYPE_FEE_PAYER: Signature.Type
    TYPE_SINGLE_SENDER: Signature.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ED25519_FIELD_NUMBER: _ClassVar[int]
    MULTI_ED25519_FIELD_NUMBER: _ClassVar[int]
    MULTI_AGENT_FIELD_NUMBER: _ClassVar[int]
    FEE_PAYER_FIELD_NUMBER: _ClassVar[int]
    SINGLE_SENDER_FIELD_NUMBER: _ClassVar[int]
    type: Signature.Type
    ed25519: Ed25519Signature
    multi_ed25519: MultiEd25519Signature
    multi_agent: MultiAgentSignature
    fee_payer: FeePayerSignature
    single_sender: SingleSender
    def __init__(
        self,
        type: _Optional[_Union[Signature.Type, str]] = ...,
        ed25519: _Optional[_Union[Ed25519Signature, _Mapping]] = ...,
        multi_ed25519: _Optional[_Union[MultiEd25519Signature, _Mapping]] = ...,
        multi_agent: _Optional[_Union[MultiAgentSignature, _Mapping]] = ...,
        fee_payer: _Optional[_Union[FeePayerSignature, _Mapping]] = ...,
        single_sender: _Optional[_Union[SingleSender, _Mapping]] = ...,
    ) -> None: ...

class Ed25519Signature(_message.Message):
    __slots__ = ["public_key", "signature"]
    PUBLIC_KEY_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    public_key: bytes
    signature: bytes
    def __init__(
        self, public_key: _Optional[bytes] = ..., signature: _Optional[bytes] = ...
    ) -> None: ...

class MultiEd25519Signature(_message.Message):
    __slots__ = ["public_keys", "signatures", "threshold", "public_key_indices"]
    PUBLIC_KEYS_FIELD_NUMBER: _ClassVar[int]
    SIGNATURES_FIELD_NUMBER: _ClassVar[int]
    THRESHOLD_FIELD_NUMBER: _ClassVar[int]
    PUBLIC_KEY_INDICES_FIELD_NUMBER: _ClassVar[int]
    public_keys: _containers.RepeatedScalarFieldContainer[bytes]
    signatures: _containers.RepeatedScalarFieldContainer[bytes]
    threshold: int
    public_key_indices: _containers.RepeatedScalarFieldContainer[int]
    def __init__(
        self,
        public_keys: _Optional[_Iterable[bytes]] = ...,
        signatures: _Optional[_Iterable[bytes]] = ...,
        threshold: _Optional[int] = ...,
        public_key_indices: _Optional[_Iterable[int]] = ...,
    ) -> None: ...

class MultiAgentSignature(_message.Message):
    __slots__ = ["sender", "secondary_signer_addresses", "secondary_signers"]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    SECONDARY_SIGNER_ADDRESSES_FIELD_NUMBER: _ClassVar[int]
    SECONDARY_SIGNERS_FIELD_NUMBER: _ClassVar[int]
    sender: AccountSignature
    secondary_signer_addresses: _containers.RepeatedScalarFieldContainer[str]
    secondary_signers: _containers.RepeatedCompositeFieldContainer[AccountSignature]
    def __init__(
        self,
        sender: _Optional[_Union[AccountSignature, _Mapping]] = ...,
        secondary_signer_addresses: _Optional[_Iterable[str]] = ...,
        secondary_signers: _Optional[
            _Iterable[_Union[AccountSignature, _Mapping]]
        ] = ...,
    ) -> None: ...

class FeePayerSignature(_message.Message):
    __slots__ = [
        "sender",
        "secondary_signer_addresses",
        "secondary_signers",
        "fee_payer_address",
        "fee_payer_signer",
    ]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    SECONDARY_SIGNER_ADDRESSES_FIELD_NUMBER: _ClassVar[int]
    SECONDARY_SIGNERS_FIELD_NUMBER: _ClassVar[int]
    FEE_PAYER_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    FEE_PAYER_SIGNER_FIELD_NUMBER: _ClassVar[int]
    sender: AccountSignature
    secondary_signer_addresses: _containers.RepeatedScalarFieldContainer[str]
    secondary_signers: _containers.RepeatedCompositeFieldContainer[AccountSignature]
    fee_payer_address: str
    fee_payer_signer: AccountSignature
    def __init__(
        self,
        sender: _Optional[_Union[AccountSignature, _Mapping]] = ...,
        secondary_signer_addresses: _Optional[_Iterable[str]] = ...,
        secondary_signers: _Optional[
            _Iterable[_Union[AccountSignature, _Mapping]]
        ] = ...,
        fee_payer_address: _Optional[str] = ...,
        fee_payer_signer: _Optional[_Union[AccountSignature, _Mapping]] = ...,
    ) -> None: ...

class AnyPublicKey(_message.Message):
    __slots__ = ["type", "public_key"]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[AnyPublicKey.Type]
        TYPE_ED25519: _ClassVar[AnyPublicKey.Type]
        TYPE_SECP256K1_ECDSA: _ClassVar[AnyPublicKey.Type]
        TYPE_SECP256R1_ECDSA: _ClassVar[AnyPublicKey.Type]
    TYPE_UNSPECIFIED: AnyPublicKey.Type
    TYPE_ED25519: AnyPublicKey.Type
    TYPE_SECP256K1_ECDSA: AnyPublicKey.Type
    TYPE_SECP256R1_ECDSA: AnyPublicKey.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PUBLIC_KEY_FIELD_NUMBER: _ClassVar[int]
    type: AnyPublicKey.Type
    public_key: bytes
    def __init__(
        self,
        type: _Optional[_Union[AnyPublicKey.Type, str]] = ...,
        public_key: _Optional[bytes] = ...,
    ) -> None: ...

class AnySignature(_message.Message):
    __slots__ = ["type", "ed25519", "secp256k1_ecdsa", "webauthn"]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[AnySignature.Type]
        TYPE_ED25519: _ClassVar[AnySignature.Type]
        TYPE_SECP256K1_ECDSA: _ClassVar[AnySignature.Type]
        TYPE_WEBAUTHN: _ClassVar[AnySignature.Type]
    TYPE_UNSPECIFIED: AnySignature.Type
    TYPE_ED25519: AnySignature.Type
    TYPE_SECP256K1_ECDSA: AnySignature.Type
    TYPE_WEBAUTHN: AnySignature.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ED25519_FIELD_NUMBER: _ClassVar[int]
    SECP256K1_ECDSA_FIELD_NUMBER: _ClassVar[int]
    WEBAUTHN_FIELD_NUMBER: _ClassVar[int]
    type: AnySignature.Type
    ed25519: Ed25519
    secp256k1_ecdsa: Secp256k1Ecdsa
    webauthn: WebAuthn
    def __init__(
        self,
        type: _Optional[_Union[AnySignature.Type, str]] = ...,
        ed25519: _Optional[_Union[Ed25519, _Mapping]] = ...,
        secp256k1_ecdsa: _Optional[_Union[Secp256k1Ecdsa, _Mapping]] = ...,
        webauthn: _Optional[_Union[WebAuthn, _Mapping]] = ...,
    ) -> None: ...

class Ed25519(_message.Message):
    __slots__ = ["signature"]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    signature: bytes
    def __init__(self, signature: _Optional[bytes] = ...) -> None: ...

class Secp256k1Ecdsa(_message.Message):
    __slots__ = ["signature"]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    signature: bytes
    def __init__(self, signature: _Optional[bytes] = ...) -> None: ...

class WebAuthn(_message.Message):
    __slots__ = ["signature"]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    signature: bytes
    def __init__(self, signature: _Optional[bytes] = ...) -> None: ...

class SingleKeySignature(_message.Message):
    __slots__ = ["public_key", "signature"]
    PUBLIC_KEY_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    public_key: AnyPublicKey
    signature: AnySignature
    def __init__(
        self,
        public_key: _Optional[_Union[AnyPublicKey, _Mapping]] = ...,
        signature: _Optional[_Union[AnySignature, _Mapping]] = ...,
    ) -> None: ...

class IndexedSignature(_message.Message):
    __slots__ = ["index", "signature"]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    index: int
    signature: AnySignature
    def __init__(
        self,
        index: _Optional[int] = ...,
        signature: _Optional[_Union[AnySignature, _Mapping]] = ...,
    ) -> None: ...

class MultiKeySignature(_message.Message):
    __slots__ = ["public_keys", "signatures", "signatures_required"]
    PUBLIC_KEYS_FIELD_NUMBER: _ClassVar[int]
    SIGNATURES_FIELD_NUMBER: _ClassVar[int]
    SIGNATURES_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    public_keys: _containers.RepeatedCompositeFieldContainer[AnyPublicKey]
    signatures: _containers.RepeatedCompositeFieldContainer[IndexedSignature]
    signatures_required: int
    def __init__(
        self,
        public_keys: _Optional[_Iterable[_Union[AnyPublicKey, _Mapping]]] = ...,
        signatures: _Optional[_Iterable[_Union[IndexedSignature, _Mapping]]] = ...,
        signatures_required: _Optional[int] = ...,
    ) -> None: ...

class SingleSender(_message.Message):
    __slots__ = ["sender"]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    sender: AccountSignature
    def __init__(
        self, sender: _Optional[_Union[AccountSignature, _Mapping]] = ...
    ) -> None: ...

class AccountSignature(_message.Message):
    __slots__ = [
        "type",
        "ed25519",
        "multi_ed25519",
        "single_key_signature",
        "multi_key_signature",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[AccountSignature.Type]
        TYPE_ED25519: _ClassVar[AccountSignature.Type]
        TYPE_MULTI_ED25519: _ClassVar[AccountSignature.Type]
        TYPE_SINGLE_KEY: _ClassVar[AccountSignature.Type]
        TYPE_MULTI_KEY: _ClassVar[AccountSignature.Type]
    TYPE_UNSPECIFIED: AccountSignature.Type
    TYPE_ED25519: AccountSignature.Type
    TYPE_MULTI_ED25519: AccountSignature.Type
    TYPE_SINGLE_KEY: AccountSignature.Type
    TYPE_MULTI_KEY: AccountSignature.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ED25519_FIELD_NUMBER: _ClassVar[int]
    MULTI_ED25519_FIELD_NUMBER: _ClassVar[int]
    SINGLE_KEY_SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    MULTI_KEY_SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    type: AccountSignature.Type
    ed25519: Ed25519Signature
    multi_ed25519: MultiEd25519Signature
    single_key_signature: SingleKeySignature
    multi_key_signature: MultiKeySignature
    def __init__(
        self,
        type: _Optional[_Union[AccountSignature.Type, str]] = ...,
        ed25519: _Optional[_Union[Ed25519Signature, _Mapping]] = ...,
        multi_ed25519: _Optional[_Union[MultiEd25519Signature, _Mapping]] = ...,
        single_key_signature: _Optional[_Union[SingleKeySignature, _Mapping]] = ...,
        multi_key_signature: _Optional[_Union[MultiKeySignature, _Mapping]] = ...,
    ) -> None: ...
