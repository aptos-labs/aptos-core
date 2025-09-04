from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from velor.indexer.v1 import filter_pb2 as _filter_pb2
from velor.transaction.v1 import transaction_pb2 as _transaction_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers

DESCRIPTOR: _descriptor.FileDescriptor

class TransactionsInStorage(_message.Message):
    __slots__ = ["transactions", "starting_version"]
    TRANSACTIONS_FIELD_NUMBER: _ClassVar[int]
    STARTING_VERSION_FIELD_NUMBER: _ClassVar[int]
    transactions: _containers.RepeatedCompositeFieldContainer[
        _transaction_pb2.Transaction
    ]
    starting_version: int
    def __init__(
        self,
        transactions: _Optional[
            _Iterable[_Union[_transaction_pb2.Transaction, _Mapping]]
        ] = ...,
        starting_version: _Optional[int] = ...,
    ) -> None: ...

class GetTransactionsRequest(_message.Message):
    __slots__ = [
        "starting_version",
        "transactions_count",
        "batch_size",
        "transaction_filter",
    ]
    STARTING_VERSION_FIELD_NUMBER: _ClassVar[int]
    TRANSACTIONS_COUNT_FIELD_NUMBER: _ClassVar[int]
    BATCH_SIZE_FIELD_NUMBER: _ClassVar[int]
    TRANSACTION_FILTER_FIELD_NUMBER: _ClassVar[int]
    starting_version: int
    transactions_count: int
    batch_size: int
    transaction_filter: _filter_pb2.BooleanTransactionFilter
    def __init__(
        self,
        starting_version: _Optional[int] = ...,
        transactions_count: _Optional[int] = ...,
        batch_size: _Optional[int] = ...,
        transaction_filter: _Optional[
            _Union[_filter_pb2.BooleanTransactionFilter, _Mapping]
        ] = ...,
    ) -> None: ...

class ProcessedRange(_message.Message):
    __slots__ = ["first_version", "last_version"]
    FIRST_VERSION_FIELD_NUMBER: _ClassVar[int]
    LAST_VERSION_FIELD_NUMBER: _ClassVar[int]
    first_version: int
    last_version: int
    def __init__(
        self, first_version: _Optional[int] = ..., last_version: _Optional[int] = ...
    ) -> None: ...

class TransactionsResponse(_message.Message):
    __slots__ = ["transactions", "chain_id", "processed_range"]
    TRANSACTIONS_FIELD_NUMBER: _ClassVar[int]
    CHAIN_ID_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_RANGE_FIELD_NUMBER: _ClassVar[int]
    transactions: _containers.RepeatedCompositeFieldContainer[
        _transaction_pb2.Transaction
    ]
    chain_id: int
    processed_range: ProcessedRange
    def __init__(
        self,
        transactions: _Optional[
            _Iterable[_Union[_transaction_pb2.Transaction, _Mapping]]
        ] = ...,
        chain_id: _Optional[int] = ...,
        processed_range: _Optional[_Union[ProcessedRange, _Mapping]] = ...,
    ) -> None: ...
