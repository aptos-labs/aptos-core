# Generated by the gRPC Python protocol compiler plugin. DO NOT EDIT!
"""Client and server classes corresponding to protobuf-defined services."""
import grpc
from aptos.internal.fullnode.v1 import (
    fullnode_data_pb2 as aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2,
)


class FullnodeDataStub(object):
    """Missing associated documentation comment in .proto file."""

    def __init__(self, channel):
        """Constructor.

        Args:
            channel: A grpc.Channel.
        """
        self.Ping = channel.unary_unary(
            "/aptos.internal.fullnode.v1.FullnodeData/Ping",
            request_serializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeRequest.SerializeToString,
            response_deserializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeResponse.FromString,
        )
        self.GetTransactionsFromNode = channel.unary_stream(
            "/aptos.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode",
            request_serializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.GetTransactionsFromNodeRequest.SerializeToString,
            response_deserializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.TransactionsFromNodeResponse.FromString,
        )


class FullnodeDataServicer(object):
    """Missing associated documentation comment in .proto file."""

    def Ping(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Method not implemented!")
        raise NotImplementedError("Method not implemented!")

    def GetTransactionsFromNode(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Method not implemented!")
        raise NotImplementedError("Method not implemented!")


def add_FullnodeDataServicer_to_server(servicer, server):
    rpc_method_handlers = {
        "Ping": grpc.unary_unary_rpc_method_handler(
            servicer.Ping,
            request_deserializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeRequest.FromString,
            response_serializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeResponse.SerializeToString,
        ),
        "GetTransactionsFromNode": grpc.unary_stream_rpc_method_handler(
            servicer.GetTransactionsFromNode,
            request_deserializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.GetTransactionsFromNodeRequest.FromString,
            response_serializer=aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.TransactionsFromNodeResponse.SerializeToString,
        ),
    }
    generic_handler = grpc.method_handlers_generic_handler(
        "aptos.internal.fullnode.v1.FullnodeData", rpc_method_handlers
    )
    server.add_generic_rpc_handlers((generic_handler,))


# This class is part of an EXPERIMENTAL API.
class FullnodeData(object):
    """Missing associated documentation comment in .proto file."""

    @staticmethod
    def Ping(
        request,
        target,
        options=(),
        channel_credentials=None,
        call_credentials=None,
        insecure=False,
        compression=None,
        wait_for_ready=None,
        timeout=None,
        metadata=None,
    ):
        return grpc.experimental.unary_unary(
            request,
            target,
            "/aptos.internal.fullnode.v1.FullnodeData/Ping",
            aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeRequest.SerializeToString,
            aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.PingFullnodeResponse.FromString,
            options,
            channel_credentials,
            insecure,
            call_credentials,
            compression,
            wait_for_ready,
            timeout,
            metadata,
        )

    @staticmethod
    def GetTransactionsFromNode(
        request,
        target,
        options=(),
        channel_credentials=None,
        call_credentials=None,
        insecure=False,
        compression=None,
        wait_for_ready=None,
        timeout=None,
        metadata=None,
    ):
        return grpc.experimental.unary_stream(
            request,
            target,
            "/aptos.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode",
            aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.GetTransactionsFromNodeRequest.SerializeToString,
            aptos_dot_internal_dot_fullnode_dot_v1_dot_fullnode__data__pb2.TransactionsFromNodeResponse.FromString,
            options,
            channel_credentials,
            insecure,
            call_credentials,
            compression,
            wait_for_ready,
            timeout,
            metadata,
        )
