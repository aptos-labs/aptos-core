from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from velor.util.timestamp import timestamp_pb2 as _timestamp_pb2
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
        "block_epilogue",
        "size_info",
    ]

    class TransactionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TRANSACTION_TYPE_UNSPECIFIED: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_GENESIS: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_BLOCK_METADATA: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_STATE_CHECKPOINT: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_USER: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_VALIDATOR: _ClassVar[Transaction.TransactionType]
        TRANSACTION_TYPE_BLOCK_EPILOGUE: _ClassVar[Transaction.TransactionType]
    TRANSACTION_TYPE_UNSPECIFIED: Transaction.TransactionType
    TRANSACTION_TYPE_GENESIS: Transaction.TransactionType
    TRANSACTION_TYPE_BLOCK_METADATA: Transaction.TransactionType
    TRANSACTION_TYPE_STATE_CHECKPOINT: Transaction.TransactionType
    TRANSACTION_TYPE_USER: Transaction.TransactionType
    TRANSACTION_TYPE_VALIDATOR: Transaction.TransactionType
    TRANSACTION_TYPE_BLOCK_EPILOGUE: Transaction.TransactionType
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
    BLOCK_EPILOGUE_FIELD_NUMBER: _ClassVar[int]
    SIZE_INFO_FIELD_NUMBER: _ClassVar[int]
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
    block_epilogue: BlockEpilogueTransaction
    size_info: TransactionSizeInfo
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
        block_epilogue: _Optional[_Union[BlockEpilogueTransaction, _Mapping]] = ...,
        size_info: _Optional[_Union[TransactionSizeInfo, _Mapping]] = ...,
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
    __slots__ = ["observed_jwk_update", "dkg_update", "events"]

    class ObservedJwkUpdate(_message.Message):
        __slots__ = ["quorum_certified_update"]

        class ExportedProviderJWKs(_message.Message):
            __slots__ = ["issuer", "version", "jwks"]

            class JWK(_message.Message):
                __slots__ = ["unsupported_jwk", "rsa"]

                class RSA(_message.Message):
                    __slots__ = ["kid", "kty", "alg", "e", "n"]
                    KID_FIELD_NUMBER: _ClassVar[int]
                    KTY_FIELD_NUMBER: _ClassVar[int]
                    ALG_FIELD_NUMBER: _ClassVar[int]
                    E_FIELD_NUMBER: _ClassVar[int]
                    N_FIELD_NUMBER: _ClassVar[int]
                    kid: str
                    kty: str
                    alg: str
                    e: str
                    n: str
                    def __init__(
                        self,
                        kid: _Optional[str] = ...,
                        kty: _Optional[str] = ...,
                        alg: _Optional[str] = ...,
                        e: _Optional[str] = ...,
                        n: _Optional[str] = ...,
                    ) -> None: ...

                class UnsupportedJWK(_message.Message):
                    __slots__ = ["id", "payload"]
                    ID_FIELD_NUMBER: _ClassVar[int]
                    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
                    id: bytes
                    payload: bytes
                    def __init__(
                        self,
                        id: _Optional[bytes] = ...,
                        payload: _Optional[bytes] = ...,
                    ) -> None: ...
                UNSUPPORTED_JWK_FIELD_NUMBER: _ClassVar[int]
                RSA_FIELD_NUMBER: _ClassVar[int]
                unsupported_jwk: ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWK
                rsa: ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSA
                def __init__(
                    self,
                    unsupported_jwk: _Optional[
                        _Union[
                            ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWK,
                            _Mapping,
                        ]
                    ] = ...,
                    rsa: _Optional[
                        _Union[
                            ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSA,
                            _Mapping,
                        ]
                    ] = ...,
                ) -> None: ...
            ISSUER_FIELD_NUMBER: _ClassVar[int]
            VERSION_FIELD_NUMBER: _ClassVar[int]
            JWKS_FIELD_NUMBER: _ClassVar[int]
            issuer: str
            version: int
            jwks: _containers.RepeatedCompositeFieldContainer[
                ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK
            ]
            def __init__(
                self,
                issuer: _Optional[str] = ...,
                version: _Optional[int] = ...,
                jwks: _Optional[
                    _Iterable[
                        _Union[
                            ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK,
                            _Mapping,
                        ]
                    ]
                ] = ...,
            ) -> None: ...

        class ExportedAggregateSignature(_message.Message):
            __slots__ = ["signer_indices", "sig"]
            SIGNER_INDICES_FIELD_NUMBER: _ClassVar[int]
            SIG_FIELD_NUMBER: _ClassVar[int]
            signer_indices: _containers.RepeatedScalarFieldContainer[int]
            sig: bytes
            def __init__(
                self,
                signer_indices: _Optional[_Iterable[int]] = ...,
                sig: _Optional[bytes] = ...,
            ) -> None: ...

        class QuorumCertifiedUpdate(_message.Message):
            __slots__ = ["update", "multi_sig"]
            UPDATE_FIELD_NUMBER: _ClassVar[int]
            MULTI_SIG_FIELD_NUMBER: _ClassVar[int]
            update: ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs
            multi_sig: ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature
            def __init__(
                self,
                update: _Optional[
                    _Union[
                        ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs,
                        _Mapping,
                    ]
                ] = ...,
                multi_sig: _Optional[
                    _Union[
                        ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature,
                        _Mapping,
                    ]
                ] = ...,
            ) -> None: ...
        QUORUM_CERTIFIED_UPDATE_FIELD_NUMBER: _ClassVar[int]
        quorum_certified_update: ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate
        def __init__(
            self,
            quorum_certified_update: _Optional[
                _Union[
                    ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate,
                    _Mapping,
                ]
            ] = ...,
        ) -> None: ...

    class DkgUpdate(_message.Message):
        __slots__ = ["dkg_transcript"]

        class DkgTranscript(_message.Message):
            __slots__ = ["epoch", "author", "payload"]
            EPOCH_FIELD_NUMBER: _ClassVar[int]
            AUTHOR_FIELD_NUMBER: _ClassVar[int]
            PAYLOAD_FIELD_NUMBER: _ClassVar[int]
            epoch: int
            author: str
            payload: bytes
            def __init__(
                self,
                epoch: _Optional[int] = ...,
                author: _Optional[str] = ...,
                payload: _Optional[bytes] = ...,
            ) -> None: ...
        DKG_TRANSCRIPT_FIELD_NUMBER: _ClassVar[int]
        dkg_transcript: ValidatorTransaction.DkgUpdate.DkgTranscript
        def __init__(
            self,
            dkg_transcript: _Optional[
                _Union[ValidatorTransaction.DkgUpdate.DkgTranscript, _Mapping]
            ] = ...,
        ) -> None: ...
    OBSERVED_JWK_UPDATE_FIELD_NUMBER: _ClassVar[int]
    DKG_UPDATE_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    observed_jwk_update: ValidatorTransaction.ObservedJwkUpdate
    dkg_update: ValidatorTransaction.DkgUpdate
    events: _containers.RepeatedCompositeFieldContainer[Event]
    def __init__(
        self,
        observed_jwk_update: _Optional[
            _Union[ValidatorTransaction.ObservedJwkUpdate, _Mapping]
        ] = ...,
        dkg_update: _Optional[_Union[ValidatorTransaction.DkgUpdate, _Mapping]] = ...,
        events: _Optional[_Iterable[_Union[Event, _Mapping]]] = ...,
    ) -> None: ...

class BlockEpilogueTransaction(_message.Message):
    __slots__ = ["block_end_info"]
    BLOCK_END_INFO_FIELD_NUMBER: _ClassVar[int]
    block_end_info: BlockEndInfo
    def __init__(
        self, block_end_info: _Optional[_Union[BlockEndInfo, _Mapping]] = ...
    ) -> None: ...

class BlockEndInfo(_message.Message):
    __slots__ = [
        "block_gas_limit_reached",
        "block_output_limit_reached",
        "block_effective_block_gas_units",
        "block_approx_output_size",
    ]
    BLOCK_GAS_LIMIT_REACHED_FIELD_NUMBER: _ClassVar[int]
    BLOCK_OUTPUT_LIMIT_REACHED_FIELD_NUMBER: _ClassVar[int]
    BLOCK_EFFECTIVE_BLOCK_GAS_UNITS_FIELD_NUMBER: _ClassVar[int]
    BLOCK_APPROX_OUTPUT_SIZE_FIELD_NUMBER: _ClassVar[int]
    block_gas_limit_reached: bool
    block_output_limit_reached: bool
    block_effective_block_gas_units: int
    block_approx_output_size: int
    def __init__(
        self,
        block_gas_limit_reached: bool = ...,
        block_output_limit_reached: bool = ...,
        block_effective_block_gas_units: _Optional[int] = ...,
        block_approx_output_size: _Optional[int] = ...,
    ) -> None: ...

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
        "write_set_payload",
        "multisig_payload",
        "extra_config_v1",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[TransactionPayload.Type]
        TYPE_ENTRY_FUNCTION_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_SCRIPT_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_WRITE_SET_PAYLOAD: _ClassVar[TransactionPayload.Type]
        TYPE_MULTISIG_PAYLOAD: _ClassVar[TransactionPayload.Type]
    TYPE_UNSPECIFIED: TransactionPayload.Type
    TYPE_ENTRY_FUNCTION_PAYLOAD: TransactionPayload.Type
    TYPE_SCRIPT_PAYLOAD: TransactionPayload.Type
    TYPE_WRITE_SET_PAYLOAD: TransactionPayload.Type
    TYPE_MULTISIG_PAYLOAD: TransactionPayload.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ENTRY_FUNCTION_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    WRITE_SET_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    MULTISIG_PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    EXTRA_CONFIG_V1_FIELD_NUMBER: _ClassVar[int]
    type: TransactionPayload.Type
    entry_function_payload: EntryFunctionPayload
    script_payload: ScriptPayload
    write_set_payload: WriteSetPayload
    multisig_payload: MultisigPayload
    extra_config_v1: ExtraConfigV1
    def __init__(
        self,
        type: _Optional[_Union[TransactionPayload.Type, str]] = ...,
        entry_function_payload: _Optional[_Union[EntryFunctionPayload, _Mapping]] = ...,
        script_payload: _Optional[_Union[ScriptPayload, _Mapping]] = ...,
        write_set_payload: _Optional[_Union[WriteSetPayload, _Mapping]] = ...,
        multisig_payload: _Optional[_Union[MultisigPayload, _Mapping]] = ...,
        extra_config_v1: _Optional[_Union[ExtraConfigV1, _Mapping]] = ...,
    ) -> None: ...

class ExtraConfigV1(_message.Message):
    __slots__ = ["multisig_address", "replay_protection_nonce"]
    MULTISIG_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    REPLAY_PROTECTION_NONCE_FIELD_NUMBER: _ClassVar[int]
    multisig_address: str
    replay_protection_nonce: int
    def __init__(
        self,
        multisig_address: _Optional[str] = ...,
        replay_protection_nonce: _Optional[int] = ...,
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
    __slots__ = [
        "name",
        "is_native",
        "is_event",
        "abilities",
        "generic_type_params",
        "fields",
    ]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IS_NATIVE_FIELD_NUMBER: _ClassVar[int]
    IS_EVENT_FIELD_NUMBER: _ClassVar[int]
    ABILITIES_FIELD_NUMBER: _ClassVar[int]
    GENERIC_TYPE_PARAMS_FIELD_NUMBER: _ClassVar[int]
    FIELDS_FIELD_NUMBER: _ClassVar[int]
    name: str
    is_native: bool
    is_event: bool
    abilities: _containers.RepeatedScalarFieldContainer[MoveAbility]
    generic_type_params: _containers.RepeatedCompositeFieldContainer[
        MoveStructGenericTypeParam
    ]
    fields: _containers.RepeatedCompositeFieldContainer[MoveStructField]
    def __init__(
        self,
        name: _Optional[str] = ...,
        is_native: bool = ...,
        is_event: bool = ...,
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
        TYPE_KEYLESS: _ClassVar[AnyPublicKey.Type]
        TYPE_FEDERATED_KEYLESS: _ClassVar[AnyPublicKey.Type]
    TYPE_UNSPECIFIED: AnyPublicKey.Type
    TYPE_ED25519: AnyPublicKey.Type
    TYPE_SECP256K1_ECDSA: AnyPublicKey.Type
    TYPE_SECP256R1_ECDSA: AnyPublicKey.Type
    TYPE_KEYLESS: AnyPublicKey.Type
    TYPE_FEDERATED_KEYLESS: AnyPublicKey.Type
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
    __slots__ = [
        "type",
        "signature",
        "ed25519",
        "secp256k1_ecdsa",
        "webauthn",
        "keyless",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[AnySignature.Type]
        TYPE_ED25519: _ClassVar[AnySignature.Type]
        TYPE_SECP256K1_ECDSA: _ClassVar[AnySignature.Type]
        TYPE_WEBAUTHN: _ClassVar[AnySignature.Type]
        TYPE_KEYLESS: _ClassVar[AnySignature.Type]
    TYPE_UNSPECIFIED: AnySignature.Type
    TYPE_ED25519: AnySignature.Type
    TYPE_SECP256K1_ECDSA: AnySignature.Type
    TYPE_WEBAUTHN: AnySignature.Type
    TYPE_KEYLESS: AnySignature.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    ED25519_FIELD_NUMBER: _ClassVar[int]
    SECP256K1_ECDSA_FIELD_NUMBER: _ClassVar[int]
    WEBAUTHN_FIELD_NUMBER: _ClassVar[int]
    KEYLESS_FIELD_NUMBER: _ClassVar[int]
    type: AnySignature.Type
    signature: bytes
    ed25519: Ed25519
    secp256k1_ecdsa: Secp256k1Ecdsa
    webauthn: WebAuthn
    keyless: Keyless
    def __init__(
        self,
        type: _Optional[_Union[AnySignature.Type, str]] = ...,
        signature: _Optional[bytes] = ...,
        ed25519: _Optional[_Union[Ed25519, _Mapping]] = ...,
        secp256k1_ecdsa: _Optional[_Union[Secp256k1Ecdsa, _Mapping]] = ...,
        webauthn: _Optional[_Union[WebAuthn, _Mapping]] = ...,
        keyless: _Optional[_Union[Keyless, _Mapping]] = ...,
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

class Keyless(_message.Message):
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

class AbstractionSignature(_message.Message):
    __slots__ = ["function_info", "signature"]
    FUNCTION_INFO_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    function_info: str
    signature: bytes
    def __init__(
        self, function_info: _Optional[str] = ..., signature: _Optional[bytes] = ...
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
        "abstraction",
    ]

    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = []
        TYPE_UNSPECIFIED: _ClassVar[AccountSignature.Type]
        TYPE_ED25519: _ClassVar[AccountSignature.Type]
        TYPE_MULTI_ED25519: _ClassVar[AccountSignature.Type]
        TYPE_SINGLE_KEY: _ClassVar[AccountSignature.Type]
        TYPE_MULTI_KEY: _ClassVar[AccountSignature.Type]
        TYPE_ABSTRACTION: _ClassVar[AccountSignature.Type]
    TYPE_UNSPECIFIED: AccountSignature.Type
    TYPE_ED25519: AccountSignature.Type
    TYPE_MULTI_ED25519: AccountSignature.Type
    TYPE_SINGLE_KEY: AccountSignature.Type
    TYPE_MULTI_KEY: AccountSignature.Type
    TYPE_ABSTRACTION: AccountSignature.Type
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ED25519_FIELD_NUMBER: _ClassVar[int]
    MULTI_ED25519_FIELD_NUMBER: _ClassVar[int]
    SINGLE_KEY_SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    MULTI_KEY_SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    ABSTRACTION_FIELD_NUMBER: _ClassVar[int]
    type: AccountSignature.Type
    ed25519: Ed25519Signature
    multi_ed25519: MultiEd25519Signature
    single_key_signature: SingleKeySignature
    multi_key_signature: MultiKeySignature
    abstraction: AbstractionSignature
    def __init__(
        self,
        type: _Optional[_Union[AccountSignature.Type, str]] = ...,
        ed25519: _Optional[_Union[Ed25519Signature, _Mapping]] = ...,
        multi_ed25519: _Optional[_Union[MultiEd25519Signature, _Mapping]] = ...,
        single_key_signature: _Optional[_Union[SingleKeySignature, _Mapping]] = ...,
        multi_key_signature: _Optional[_Union[MultiKeySignature, _Mapping]] = ...,
        abstraction: _Optional[_Union[AbstractionSignature, _Mapping]] = ...,
    ) -> None: ...

class TransactionSizeInfo(_message.Message):
    __slots__ = ["transaction_bytes", "event_size_info", "write_op_size_info"]
    TRANSACTION_BYTES_FIELD_NUMBER: _ClassVar[int]
    EVENT_SIZE_INFO_FIELD_NUMBER: _ClassVar[int]
    WRITE_OP_SIZE_INFO_FIELD_NUMBER: _ClassVar[int]
    transaction_bytes: int
    event_size_info: _containers.RepeatedCompositeFieldContainer[EventSizeInfo]
    write_op_size_info: _containers.RepeatedCompositeFieldContainer[WriteOpSizeInfo]
    def __init__(
        self,
        transaction_bytes: _Optional[int] = ...,
        event_size_info: _Optional[_Iterable[_Union[EventSizeInfo, _Mapping]]] = ...,
        write_op_size_info: _Optional[
            _Iterable[_Union[WriteOpSizeInfo, _Mapping]]
        ] = ...,
    ) -> None: ...

class EventSizeInfo(_message.Message):
    __slots__ = ["type_tag_bytes", "total_bytes"]
    TYPE_TAG_BYTES_FIELD_NUMBER: _ClassVar[int]
    TOTAL_BYTES_FIELD_NUMBER: _ClassVar[int]
    type_tag_bytes: int
    total_bytes: int
    def __init__(
        self, type_tag_bytes: _Optional[int] = ..., total_bytes: _Optional[int] = ...
    ) -> None: ...

class WriteOpSizeInfo(_message.Message):
    __slots__ = ["key_bytes", "value_bytes"]
    KEY_BYTES_FIELD_NUMBER: _ClassVar[int]
    VALUE_BYTES_FIELD_NUMBER: _ClassVar[int]
    key_bytes: int
    value_bytes: int
    def __init__(
        self, key_bytes: _Optional[int] = ..., value_bytes: _Optional[int] = ...
    ) -> None: ...
