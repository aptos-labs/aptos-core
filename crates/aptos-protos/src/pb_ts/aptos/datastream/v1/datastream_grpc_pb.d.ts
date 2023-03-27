// package: aptos.datastream.v1
// file: aptos/datastream/v1/datastream.proto

/* tslint:disable */
/* eslint-disable */

import * as grpc from "@grpc/grpc-js";
import * as aptos_datastream_v1_datastream_pb from "../../../aptos/datastream/v1/datastream_pb";
import * as aptos_util_timestamp_timestamp_pb from "../../../aptos/util/timestamp/timestamp_pb";

interface IIndexerStreamService extends grpc.ServiceDefinition<grpc.UntypedServiceImplementation> {
    rawDatastream: IIndexerStreamService_IRawDatastream;
}

interface IIndexerStreamService_IRawDatastream extends grpc.MethodDefinition<aptos_datastream_v1_datastream_pb.RawDatastreamRequest, aptos_datastream_v1_datastream_pb.RawDatastreamResponse> {
    path: "/aptos.datastream.v1.IndexerStream/RawDatastream";
    requestStream: false;
    responseStream: true;
    requestSerialize: grpc.serialize<aptos_datastream_v1_datastream_pb.RawDatastreamRequest>;
    requestDeserialize: grpc.deserialize<aptos_datastream_v1_datastream_pb.RawDatastreamRequest>;
    responseSerialize: grpc.serialize<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
    responseDeserialize: grpc.deserialize<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
}

export const IndexerStreamService: IIndexerStreamService;

export interface IIndexerStreamServer extends grpc.UntypedServiceImplementation {
    rawDatastream: grpc.handleServerStreamingCall<aptos_datastream_v1_datastream_pb.RawDatastreamRequest, aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
}

export interface IIndexerStreamClient {
    rawDatastream(request: aptos_datastream_v1_datastream_pb.RawDatastreamRequest, options?: Partial<grpc.CallOptions>): grpc.ClientReadableStream<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
    rawDatastream(request: aptos_datastream_v1_datastream_pb.RawDatastreamRequest, metadata?: grpc.Metadata, options?: Partial<grpc.CallOptions>): grpc.ClientReadableStream<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
}

export class IndexerStreamClient extends grpc.Client implements IIndexerStreamClient {
    constructor(address: string, credentials: grpc.ChannelCredentials, options?: Partial<grpc.ClientOptions>);
    public rawDatastream(request: aptos_datastream_v1_datastream_pb.RawDatastreamRequest, options?: Partial<grpc.CallOptions>): grpc.ClientReadableStream<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
    public rawDatastream(request: aptos_datastream_v1_datastream_pb.RawDatastreamRequest, metadata?: grpc.Metadata, options?: Partial<grpc.CallOptions>): grpc.ClientReadableStream<aptos_datastream_v1_datastream_pb.RawDatastreamResponse>;
}
