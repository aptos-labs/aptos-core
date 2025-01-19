# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: aptos/internal/fullnode/v1/fullnode_data.proto
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder

# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


from aptos.indexer.v1 import grpc_pb2 as aptos_dot_indexer_dot_v1_dot_grpc__pb2
from aptos.transaction.v1 import (
    transaction_pb2 as aptos_dot_transaction_dot_v1_dot_transaction__pb2,
)

DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(
    b'\n.aptos/internal/fullnode/v1/fullnode_data.proto\x12\x1a\x61ptos.internal.fullnode.v1\x1a&aptos/transaction/v1/transaction.proto\x1a\x1b\x61ptos/indexer/v1/grpc.proto"M\n\x12TransactionsOutput\x12\x37\n\x0ctransactions\x18\x01 \x03(\x0b\x32!.aptos.transaction.v1.Transaction"\xf2\x01\n\x0cStreamStatus\x12\x41\n\x04type\x18\x01 \x01(\x0e\x32\x33.aptos.internal.fullnode.v1.StreamStatus.StatusType\x12\x15\n\rstart_version\x18\x02 \x01(\x04\x12\x1c\n\x0b\x65nd_version\x18\x03 \x01(\x04\x42\x02\x30\x01H\x00\x88\x01\x01"Z\n\nStatusType\x12\x1b\n\x17STATUS_TYPE_UNSPECIFIED\x10\x00\x12\x14\n\x10STATUS_TYPE_INIT\x10\x01\x12\x19\n\x15STATUS_TYPE_BATCH_END\x10\x02\x42\x0e\n\x0c_end_version"\x94\x01\n\x1eGetTransactionsFromNodeRequest\x12!\n\x10starting_version\x18\x01 \x01(\x04\x42\x02\x30\x01H\x00\x88\x01\x01\x12#\n\x12transactions_count\x18\x02 \x01(\x04\x42\x02\x30\x01H\x01\x88\x01\x01\x42\x13\n\x11_starting_versionB\x15\n\x13_transactions_count"\xb8\x01\n\x1cTransactionsFromNodeResponse\x12:\n\x06status\x18\x01 \x01(\x0b\x32(.aptos.internal.fullnode.v1.StreamStatusH\x00\x12>\n\x04\x64\x61ta\x18\x02 \x01(\x0b\x32..aptos.internal.fullnode.v1.TransactionsOutputH\x00\x12\x10\n\x08\x63hain_id\x18\x03 \x01(\rB\n\n\x08response"\x15\n\x13PingFullnodeRequest"R\n\x14PingFullnodeResponse\x12\x31\n\x04info\x18\x01 \x01(\x0b\x32\x1e.aptos.indexer.v1.FullnodeInfoH\x00\x88\x01\x01\x42\x07\n\x05_info2\x8d\x02\n\x0c\x46ullnodeData\x12i\n\x04Ping\x12/.aptos.internal.fullnode.v1.PingFullnodeRequest\x1a\x30.aptos.internal.fullnode.v1.PingFullnodeResponse\x12\x91\x01\n\x17GetTransactionsFromNode\x12:.aptos.internal.fullnode.v1.GetTransactionsFromNodeRequest\x1a\x38.aptos.internal.fullnode.v1.TransactionsFromNodeResponse0\x01\x62\x06proto3'
)

_globals = globals()
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, _globals)
_builder.BuildTopDescriptorsAndMessages(
    DESCRIPTOR, "aptos.internal.fullnode.v1.fullnode_data_pb2", _globals
)
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    _STREAMSTATUS.fields_by_name["end_version"]._options = None
    _STREAMSTATUS.fields_by_name["end_version"]._serialized_options = b"0\001"
    _GETTRANSACTIONSFROMNODEREQUEST.fields_by_name["starting_version"]._options = None
    _GETTRANSACTIONSFROMNODEREQUEST.fields_by_name[
        "starting_version"
    ]._serialized_options = b"0\001"
    _GETTRANSACTIONSFROMNODEREQUEST.fields_by_name["transactions_count"]._options = None
    _GETTRANSACTIONSFROMNODEREQUEST.fields_by_name[
        "transactions_count"
    ]._serialized_options = b"0\001"
    _globals["_TRANSACTIONSOUTPUT"]._serialized_start = 147
    _globals["_TRANSACTIONSOUTPUT"]._serialized_end = 224
    _globals["_STREAMSTATUS"]._serialized_start = 227
    _globals["_STREAMSTATUS"]._serialized_end = 469
    _globals["_STREAMSTATUS_STATUSTYPE"]._serialized_start = 363
    _globals["_STREAMSTATUS_STATUSTYPE"]._serialized_end = 453
    _globals["_GETTRANSACTIONSFROMNODEREQUEST"]._serialized_start = 472
    _globals["_GETTRANSACTIONSFROMNODEREQUEST"]._serialized_end = 620
    _globals["_TRANSACTIONSFROMNODERESPONSE"]._serialized_start = 623
    _globals["_TRANSACTIONSFROMNODERESPONSE"]._serialized_end = 807
    _globals["_PINGFULLNODEREQUEST"]._serialized_start = 809
    _globals["_PINGFULLNODEREQUEST"]._serialized_end = 830
    _globals["_PINGFULLNODERESPONSE"]._serialized_start = 832
    _globals["_PINGFULLNODERESPONSE"]._serialized_end = 914
    _globals["_FULLNODEDATA"]._serialized_start = 917
    _globals["_FULLNODEDATA"]._serialized_end = 1186
# @@protoc_insertion_point(module_scope)
