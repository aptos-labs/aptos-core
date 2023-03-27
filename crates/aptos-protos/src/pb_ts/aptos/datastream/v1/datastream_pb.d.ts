// package: aptos.datastream.v1
// file: aptos/datastream/v1/datastream.proto

/* tslint:disable */
/* eslint-disable */

import * as jspb from "google-protobuf";
import * as aptos_util_timestamp_timestamp_pb from "../../../aptos/util/timestamp/timestamp_pb";

export class TransactionsOutput extends jspb.Message { 
    clearTransactionsList(): void;
    getTransactionsList(): Array<TransactionOutput>;
    setTransactionsList(value: Array<TransactionOutput>): TransactionsOutput;
    addTransactions(value?: TransactionOutput, index?: number): TransactionOutput;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): TransactionsOutput.AsObject;
    static toObject(includeInstance: boolean, msg: TransactionsOutput): TransactionsOutput.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: TransactionsOutput, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): TransactionsOutput;
    static deserializeBinaryFromReader(message: TransactionsOutput, reader: jspb.BinaryReader): TransactionsOutput;
}

export namespace TransactionsOutput {
    export type AsObject = {
        transactionsList: Array<TransactionOutput.AsObject>,
    }
}

export class TransactionOutput extends jspb.Message { 
    getEncodedProtoData(): string;
    setEncodedProtoData(value: string): TransactionOutput;
    getVersion(): number;
    setVersion(value: number): TransactionOutput;

    hasTimestamp(): boolean;
    clearTimestamp(): void;
    getTimestamp(): aptos_util_timestamp_timestamp_pb.Timestamp | undefined;
    setTimestamp(value?: aptos_util_timestamp_timestamp_pb.Timestamp): TransactionOutput;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): TransactionOutput.AsObject;
    static toObject(includeInstance: boolean, msg: TransactionOutput): TransactionOutput.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: TransactionOutput, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): TransactionOutput;
    static deserializeBinaryFromReader(message: TransactionOutput, reader: jspb.BinaryReader): TransactionOutput;
}

export namespace TransactionOutput {
    export type AsObject = {
        encodedProtoData: string,
        version: number,
        timestamp?: aptos_util_timestamp_timestamp_pb.Timestamp.AsObject,
    }
}

export class StreamStatus extends jspb.Message { 
    getType(): StreamStatus.StatusType;
    setType(value: StreamStatus.StatusType): StreamStatus;
    getStartVersion(): number;
    setStartVersion(value: number): StreamStatus;

    hasEndVersion(): boolean;
    clearEndVersion(): void;
    getEndVersion(): number | undefined;
    setEndVersion(value: number): StreamStatus;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): StreamStatus.AsObject;
    static toObject(includeInstance: boolean, msg: StreamStatus): StreamStatus.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: StreamStatus, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): StreamStatus;
    static deserializeBinaryFromReader(message: StreamStatus, reader: jspb.BinaryReader): StreamStatus;
}

export namespace StreamStatus {
    export type AsObject = {
        type: StreamStatus.StatusType,
        startVersion: number,
        endVersion?: number,
    }

    export enum StatusType {
    STATUS_TYPE_UNSPECIFIED = 0,
    STATUS_TYPE_INIT = 1,
    STATUS_TYPE_BATCH_END = 2,
    }

}

export class RawDatastreamRequest extends jspb.Message { 

    hasStartingVersion(): boolean;
    clearStartingVersion(): void;
    getStartingVersion(): number | undefined;
    setStartingVersion(value: number): RawDatastreamRequest;

    hasTransactionsCount(): boolean;
    clearTransactionsCount(): void;
    getTransactionsCount(): number | undefined;
    setTransactionsCount(value: number): RawDatastreamRequest;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): RawDatastreamRequest.AsObject;
    static toObject(includeInstance: boolean, msg: RawDatastreamRequest): RawDatastreamRequest.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: RawDatastreamRequest, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): RawDatastreamRequest;
    static deserializeBinaryFromReader(message: RawDatastreamRequest, reader: jspb.BinaryReader): RawDatastreamRequest;
}

export namespace RawDatastreamRequest {
    export type AsObject = {
        startingVersion?: number,
        transactionsCount?: number,
    }
}

export class RawDatastreamResponse extends jspb.Message { 

    hasStatus(): boolean;
    clearStatus(): void;
    getStatus(): StreamStatus | undefined;
    setStatus(value?: StreamStatus): RawDatastreamResponse;

    hasData(): boolean;
    clearData(): void;
    getData(): TransactionsOutput | undefined;
    setData(value?: TransactionsOutput): RawDatastreamResponse;
    getChainId(): number;
    setChainId(value: number): RawDatastreamResponse;

    getResponseCase(): RawDatastreamResponse.ResponseCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): RawDatastreamResponse.AsObject;
    static toObject(includeInstance: boolean, msg: RawDatastreamResponse): RawDatastreamResponse.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: RawDatastreamResponse, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): RawDatastreamResponse;
    static deserializeBinaryFromReader(message: RawDatastreamResponse, reader: jspb.BinaryReader): RawDatastreamResponse;
}

export namespace RawDatastreamResponse {
    export type AsObject = {
        status?: StreamStatus.AsObject,
        data?: TransactionsOutput.AsObject,
        chainId: number,
    }

    export enum ResponseCase {
        RESPONSE_NOT_SET = 0,
        STATUS = 1,
        DATA = 2,
    }

}
