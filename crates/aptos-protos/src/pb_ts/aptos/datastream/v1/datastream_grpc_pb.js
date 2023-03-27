// GENERATED CODE -- DO NOT EDIT!

// Original file comments:
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//
'use strict';
var grpc = require('@grpc/grpc-js');
var aptos_datastream_v1_datastream_pb = require('../../../aptos/datastream/v1/datastream_pb.js');
var aptos_util_timestamp_timestamp_pb = require('../../../aptos/util/timestamp/timestamp_pb.js');

function serialize_aptos_datastream_v1_RawDatastreamRequest(arg) {
  if (!(arg instanceof aptos_datastream_v1_datastream_pb.RawDatastreamRequest)) {
    throw new Error('Expected argument of type aptos.datastream.v1.RawDatastreamRequest');
  }
  return Buffer.from(arg.serializeBinary());
}

function deserialize_aptos_datastream_v1_RawDatastreamRequest(buffer_arg) {
  return aptos_datastream_v1_datastream_pb.RawDatastreamRequest.deserializeBinary(new Uint8Array(buffer_arg));
}

function serialize_aptos_datastream_v1_RawDatastreamResponse(arg) {
  if (!(arg instanceof aptos_datastream_v1_datastream_pb.RawDatastreamResponse)) {
    throw new Error('Expected argument of type aptos.datastream.v1.RawDatastreamResponse');
  }
  return Buffer.from(arg.serializeBinary());
}

function deserialize_aptos_datastream_v1_RawDatastreamResponse(buffer_arg) {
  return aptos_datastream_v1_datastream_pb.RawDatastreamResponse.deserializeBinary(new Uint8Array(buffer_arg));
}


var IndexerStreamService = exports.IndexerStreamService = {
  rawDatastream: {
    path: '/aptos.datastream.v1.IndexerStream/RawDatastream',
    requestStream: false,
    responseStream: true,
    requestType: aptos_datastream_v1_datastream_pb.RawDatastreamRequest,
    responseType: aptos_datastream_v1_datastream_pb.RawDatastreamResponse,
    requestSerialize: serialize_aptos_datastream_v1_RawDatastreamRequest,
    requestDeserialize: deserialize_aptos_datastream_v1_RawDatastreamRequest,
    responseSerialize: serialize_aptos_datastream_v1_RawDatastreamResponse,
    responseDeserialize: deserialize_aptos_datastream_v1_RawDatastreamResponse,
  },
};

exports.IndexerStreamClient = grpc.makeGenericClientConstructor(IndexerStreamService);
