from typing import ClassVar as _ClassVar
from typing import Optional as _Optional

from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message

DESCRIPTOR: _descriptor.FileDescriptor

class Transaction(_message.Message):
    __slots__ = [
        "version",
        "block_height",
        "hash",
        "type",
        "payload",
        "state_change_hash",
        "event_root_hash",
        "state_checkpoint_hash",
        "gas_used",
        "success",
        "vm_status",
        "accumulator_root_hash",
        "num_events",
        "num_write_set_changes",
        "epoch",
        "inserted_at",
    ]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    BLOCK_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    HASH_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    STATE_CHANGE_HASH_FIELD_NUMBER: _ClassVar[int]
    EVENT_ROOT_HASH_FIELD_NUMBER: _ClassVar[int]
    STATE_CHECKPOINT_HASH_FIELD_NUMBER: _ClassVar[int]
    GAS_USED_FIELD_NUMBER: _ClassVar[int]
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    VM_STATUS_FIELD_NUMBER: _ClassVar[int]
    ACCUMULATOR_ROOT_HASH_FIELD_NUMBER: _ClassVar[int]
    NUM_EVENTS_FIELD_NUMBER: _ClassVar[int]
    NUM_WRITE_SET_CHANGES_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    INSERTED_AT_FIELD_NUMBER: _ClassVar[int]
    version: int
    block_height: int
    hash: str
    type: str
    payload: str
    state_change_hash: str
    event_root_hash: str
    state_checkpoint_hash: str
    gas_used: int
    success: bool
    vm_status: str
    accumulator_root_hash: str
    num_events: int
    num_write_set_changes: int
    epoch: int
    inserted_at: int
    def __init__(
        self,
        version: _Optional[int] = ...,
        block_height: _Optional[int] = ...,
        hash: _Optional[str] = ...,
        type: _Optional[str] = ...,
        payload: _Optional[str] = ...,
        state_change_hash: _Optional[str] = ...,
        event_root_hash: _Optional[str] = ...,
        state_checkpoint_hash: _Optional[str] = ...,
        gas_used: _Optional[int] = ...,
        success: bool = ...,
        vm_status: _Optional[str] = ...,
        accumulator_root_hash: _Optional[str] = ...,
        num_events: _Optional[int] = ...,
        num_write_set_changes: _Optional[int] = ...,
        epoch: _Optional[int] = ...,
        inserted_at: _Optional[int] = ...,
    ) -> None: ...
