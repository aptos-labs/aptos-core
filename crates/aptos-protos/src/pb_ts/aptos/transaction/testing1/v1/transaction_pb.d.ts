// package: aptos.transaction.testing1.v1
// file: aptos/transaction/testing1/v1/transaction.proto

/* tslint:disable */
/* eslint-disable */

import * as jspb from "google-protobuf";
import * as aptos_util_timestamp_timestamp_pb from "../../../../aptos/util/timestamp/timestamp_pb";

export class Block extends jspb.Message { 

    hasTimestamp(): boolean;
    clearTimestamp(): void;
    getTimestamp(): aptos_util_timestamp_timestamp_pb.Timestamp | undefined;
    setTimestamp(value?: aptos_util_timestamp_timestamp_pb.Timestamp): Block;
    getHeight(): number;
    setHeight(value: number): Block;
    clearTransactionsList(): void;
    getTransactionsList(): Array<Transaction>;
    setTransactionsList(value: Array<Transaction>): Block;
    addTransactions(value?: Transaction, index?: number): Transaction;
    getChainId(): number;
    setChainId(value: number): Block;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Block.AsObject;
    static toObject(includeInstance: boolean, msg: Block): Block.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Block, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Block;
    static deserializeBinaryFromReader(message: Block, reader: jspb.BinaryReader): Block;
}

export namespace Block {
    export type AsObject = {
        timestamp?: aptos_util_timestamp_timestamp_pb.Timestamp.AsObject,
        height: number,
        transactionsList: Array<Transaction.AsObject>,
        chainId: number,
    }
}

export class Transaction extends jspb.Message { 

    hasTimestamp(): boolean;
    clearTimestamp(): void;
    getTimestamp(): aptos_util_timestamp_timestamp_pb.Timestamp | undefined;
    setTimestamp(value?: aptos_util_timestamp_timestamp_pb.Timestamp): Transaction;
    getVersion(): number;
    setVersion(value: number): Transaction;

    hasInfo(): boolean;
    clearInfo(): void;
    getInfo(): TransactionInfo | undefined;
    setInfo(value?: TransactionInfo): Transaction;
    getEpoch(): number;
    setEpoch(value: number): Transaction;
    getBlockHeight(): number;
    setBlockHeight(value: number): Transaction;
    getType(): Transaction.TransactionType;
    setType(value: Transaction.TransactionType): Transaction;

    hasBlockMetadata(): boolean;
    clearBlockMetadata(): void;
    getBlockMetadata(): BlockMetadataTransaction | undefined;
    setBlockMetadata(value?: BlockMetadataTransaction): Transaction;

    hasGenesis(): boolean;
    clearGenesis(): void;
    getGenesis(): GenesisTransaction | undefined;
    setGenesis(value?: GenesisTransaction): Transaction;

    hasStateCheckpoint(): boolean;
    clearStateCheckpoint(): void;
    getStateCheckpoint(): StateCheckpointTransaction | undefined;
    setStateCheckpoint(value?: StateCheckpointTransaction): Transaction;

    hasUser(): boolean;
    clearUser(): void;
    getUser(): UserTransaction | undefined;
    setUser(value?: UserTransaction): Transaction;

    getTxnDataCase(): Transaction.TxnDataCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Transaction.AsObject;
    static toObject(includeInstance: boolean, msg: Transaction): Transaction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Transaction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Transaction;
    static deserializeBinaryFromReader(message: Transaction, reader: jspb.BinaryReader): Transaction;
}

export namespace Transaction {
    export type AsObject = {
        timestamp?: aptos_util_timestamp_timestamp_pb.Timestamp.AsObject,
        version: number,
        info?: TransactionInfo.AsObject,
        epoch: number,
        blockHeight: number,
        type: Transaction.TransactionType,
        blockMetadata?: BlockMetadataTransaction.AsObject,
        genesis?: GenesisTransaction.AsObject,
        stateCheckpoint?: StateCheckpointTransaction.AsObject,
        user?: UserTransaction.AsObject,
    }

    export enum TransactionType {
    TRANSACTION_TYPE_UNSPECIFIED = 0,
    TRANSACTION_TYPE_GENESIS = 1,
    TRANSACTION_TYPE_BLOCK_METADATA = 2,
    TRANSACTION_TYPE_STATE_CHECKPOINT = 3,
    TRANSACTION_TYPE_USER = 4,
    }


    export enum TxnDataCase {
        TXN_DATA_NOT_SET = 0,
        BLOCK_METADATA = 7,
        GENESIS = 8,
        STATE_CHECKPOINT = 9,
        USER = 10,
    }

}

export class BlockMetadataTransaction extends jspb.Message { 
    getId(): string;
    setId(value: string): BlockMetadataTransaction;
    getRound(): number;
    setRound(value: number): BlockMetadataTransaction;
    clearEventsList(): void;
    getEventsList(): Array<Event>;
    setEventsList(value: Array<Event>): BlockMetadataTransaction;
    addEvents(value?: Event, index?: number): Event;
    getPreviousBlockVotesBitvec(): Uint8Array | string;
    getPreviousBlockVotesBitvec_asU8(): Uint8Array;
    getPreviousBlockVotesBitvec_asB64(): string;
    setPreviousBlockVotesBitvec(value: Uint8Array | string): BlockMetadataTransaction;
    getProposer(): string;
    setProposer(value: string): BlockMetadataTransaction;
    clearFailedProposerIndicesList(): void;
    getFailedProposerIndicesList(): Array<number>;
    setFailedProposerIndicesList(value: Array<number>): BlockMetadataTransaction;
    addFailedProposerIndices(value: number, index?: number): number;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): BlockMetadataTransaction.AsObject;
    static toObject(includeInstance: boolean, msg: BlockMetadataTransaction): BlockMetadataTransaction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: BlockMetadataTransaction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): BlockMetadataTransaction;
    static deserializeBinaryFromReader(message: BlockMetadataTransaction, reader: jspb.BinaryReader): BlockMetadataTransaction;
}

export namespace BlockMetadataTransaction {
    export type AsObject = {
        id: string,
        round: number,
        eventsList: Array<Event.AsObject>,
        previousBlockVotesBitvec: Uint8Array | string,
        proposer: string,
        failedProposerIndicesList: Array<number>,
    }
}

export class GenesisTransaction extends jspb.Message { 

    hasPayload(): boolean;
    clearPayload(): void;
    getPayload(): WriteSet | undefined;
    setPayload(value?: WriteSet): GenesisTransaction;
    clearEventsList(): void;
    getEventsList(): Array<Event>;
    setEventsList(value: Array<Event>): GenesisTransaction;
    addEvents(value?: Event, index?: number): Event;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GenesisTransaction.AsObject;
    static toObject(includeInstance: boolean, msg: GenesisTransaction): GenesisTransaction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GenesisTransaction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GenesisTransaction;
    static deserializeBinaryFromReader(message: GenesisTransaction, reader: jspb.BinaryReader): GenesisTransaction;
}

export namespace GenesisTransaction {
    export type AsObject = {
        payload?: WriteSet.AsObject,
        eventsList: Array<Event.AsObject>,
    }
}

export class StateCheckpointTransaction extends jspb.Message { 

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): StateCheckpointTransaction.AsObject;
    static toObject(includeInstance: boolean, msg: StateCheckpointTransaction): StateCheckpointTransaction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: StateCheckpointTransaction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): StateCheckpointTransaction;
    static deserializeBinaryFromReader(message: StateCheckpointTransaction, reader: jspb.BinaryReader): StateCheckpointTransaction;
}

export namespace StateCheckpointTransaction {
    export type AsObject = {
    }
}

export class UserTransaction extends jspb.Message { 

    hasRequest(): boolean;
    clearRequest(): void;
    getRequest(): UserTransactionRequest | undefined;
    setRequest(value?: UserTransactionRequest): UserTransaction;
    clearEventsList(): void;
    getEventsList(): Array<Event>;
    setEventsList(value: Array<Event>): UserTransaction;
    addEvents(value?: Event, index?: number): Event;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): UserTransaction.AsObject;
    static toObject(includeInstance: boolean, msg: UserTransaction): UserTransaction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: UserTransaction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): UserTransaction;
    static deserializeBinaryFromReader(message: UserTransaction, reader: jspb.BinaryReader): UserTransaction;
}

export namespace UserTransaction {
    export type AsObject = {
        request?: UserTransactionRequest.AsObject,
        eventsList: Array<Event.AsObject>,
    }
}

export class Event extends jspb.Message { 

    hasKey(): boolean;
    clearKey(): void;
    getKey(): EventKey | undefined;
    setKey(value?: EventKey): Event;
    getSequenceNumber(): number;
    setSequenceNumber(value: number): Event;

    hasType(): boolean;
    clearType(): void;
    getType(): MoveType | undefined;
    setType(value?: MoveType): Event;
    getTypeStr(): string;
    setTypeStr(value: string): Event;
    getData(): string;
    setData(value: string): Event;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Event.AsObject;
    static toObject(includeInstance: boolean, msg: Event): Event.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Event, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Event;
    static deserializeBinaryFromReader(message: Event, reader: jspb.BinaryReader): Event;
}

export namespace Event {
    export type AsObject = {
        key?: EventKey.AsObject,
        sequenceNumber: number,
        type?: MoveType.AsObject,
        typeStr: string,
        data: string,
    }
}

export class TransactionInfo extends jspb.Message { 
    getHash(): Uint8Array | string;
    getHash_asU8(): Uint8Array;
    getHash_asB64(): string;
    setHash(value: Uint8Array | string): TransactionInfo;
    getStateChangeHash(): Uint8Array | string;
    getStateChangeHash_asU8(): Uint8Array;
    getStateChangeHash_asB64(): string;
    setStateChangeHash(value: Uint8Array | string): TransactionInfo;
    getEventRootHash(): Uint8Array | string;
    getEventRootHash_asU8(): Uint8Array;
    getEventRootHash_asB64(): string;
    setEventRootHash(value: Uint8Array | string): TransactionInfo;

    hasStateCheckpointHash(): boolean;
    clearStateCheckpointHash(): void;
    getStateCheckpointHash(): Uint8Array | string;
    getStateCheckpointHash_asU8(): Uint8Array;
    getStateCheckpointHash_asB64(): string;
    setStateCheckpointHash(value: Uint8Array | string): TransactionInfo;
    getGasUsed(): number;
    setGasUsed(value: number): TransactionInfo;
    getSuccess(): boolean;
    setSuccess(value: boolean): TransactionInfo;
    getVmStatus(): string;
    setVmStatus(value: string): TransactionInfo;
    getAccumulatorRootHash(): Uint8Array | string;
    getAccumulatorRootHash_asU8(): Uint8Array;
    getAccumulatorRootHash_asB64(): string;
    setAccumulatorRootHash(value: Uint8Array | string): TransactionInfo;
    clearChangesList(): void;
    getChangesList(): Array<WriteSetChange>;
    setChangesList(value: Array<WriteSetChange>): TransactionInfo;
    addChanges(value?: WriteSetChange, index?: number): WriteSetChange;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): TransactionInfo.AsObject;
    static toObject(includeInstance: boolean, msg: TransactionInfo): TransactionInfo.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: TransactionInfo, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): TransactionInfo;
    static deserializeBinaryFromReader(message: TransactionInfo, reader: jspb.BinaryReader): TransactionInfo;
}

export namespace TransactionInfo {
    export type AsObject = {
        hash: Uint8Array | string,
        stateChangeHash: Uint8Array | string,
        eventRootHash: Uint8Array | string,
        stateCheckpointHash: Uint8Array | string,
        gasUsed: number,
        success: boolean,
        vmStatus: string,
        accumulatorRootHash: Uint8Array | string,
        changesList: Array<WriteSetChange.AsObject>,
    }
}

export class EventKey extends jspb.Message { 
    getCreationNumber(): number;
    setCreationNumber(value: number): EventKey;
    getAccountAddress(): string;
    setAccountAddress(value: string): EventKey;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): EventKey.AsObject;
    static toObject(includeInstance: boolean, msg: EventKey): EventKey.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: EventKey, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): EventKey;
    static deserializeBinaryFromReader(message: EventKey, reader: jspb.BinaryReader): EventKey;
}

export namespace EventKey {
    export type AsObject = {
        creationNumber: number,
        accountAddress: string,
    }
}

export class UserTransactionRequest extends jspb.Message { 
    getSender(): string;
    setSender(value: string): UserTransactionRequest;
    getSequenceNumber(): number;
    setSequenceNumber(value: number): UserTransactionRequest;
    getMaxGasAmount(): number;
    setMaxGasAmount(value: number): UserTransactionRequest;
    getGasUnitPrice(): number;
    setGasUnitPrice(value: number): UserTransactionRequest;

    hasExpirationTimestampSecs(): boolean;
    clearExpirationTimestampSecs(): void;
    getExpirationTimestampSecs(): aptos_util_timestamp_timestamp_pb.Timestamp | undefined;
    setExpirationTimestampSecs(value?: aptos_util_timestamp_timestamp_pb.Timestamp): UserTransactionRequest;

    hasPayload(): boolean;
    clearPayload(): void;
    getPayload(): TransactionPayload | undefined;
    setPayload(value?: TransactionPayload): UserTransactionRequest;

    hasSignature(): boolean;
    clearSignature(): void;
    getSignature(): Signature | undefined;
    setSignature(value?: Signature): UserTransactionRequest;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): UserTransactionRequest.AsObject;
    static toObject(includeInstance: boolean, msg: UserTransactionRequest): UserTransactionRequest.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: UserTransactionRequest, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): UserTransactionRequest;
    static deserializeBinaryFromReader(message: UserTransactionRequest, reader: jspb.BinaryReader): UserTransactionRequest;
}

export namespace UserTransactionRequest {
    export type AsObject = {
        sender: string,
        sequenceNumber: number,
        maxGasAmount: number,
        gasUnitPrice: number,
        expirationTimestampSecs?: aptos_util_timestamp_timestamp_pb.Timestamp.AsObject,
        payload?: TransactionPayload.AsObject,
        signature?: Signature.AsObject,
    }
}

export class WriteSet extends jspb.Message { 
    getWriteSetType(): WriteSet.WriteSetType;
    setWriteSetType(value: WriteSet.WriteSetType): WriteSet;

    hasScriptWriteSet(): boolean;
    clearScriptWriteSet(): void;
    getScriptWriteSet(): ScriptWriteSet | undefined;
    setScriptWriteSet(value?: ScriptWriteSet): WriteSet;

    hasDirectWriteSet(): boolean;
    clearDirectWriteSet(): void;
    getDirectWriteSet(): DirectWriteSet | undefined;
    setDirectWriteSet(value?: DirectWriteSet): WriteSet;

    getWriteSetCase(): WriteSet.WriteSetCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteSet.AsObject;
    static toObject(includeInstance: boolean, msg: WriteSet): WriteSet.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteSet, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteSet;
    static deserializeBinaryFromReader(message: WriteSet, reader: jspb.BinaryReader): WriteSet;
}

export namespace WriteSet {
    export type AsObject = {
        writeSetType: WriteSet.WriteSetType,
        scriptWriteSet?: ScriptWriteSet.AsObject,
        directWriteSet?: DirectWriteSet.AsObject,
    }

    export enum WriteSetType {
    WRITE_SET_TYPE_UNSPECIFIED = 0,
    WRITE_SET_TYPE_SCRIPT_WRITE_SET = 1,
    WRITE_SET_TYPE_DIRECT_WRITE_SET = 2,
    }


    export enum WriteSetCase {
        WRITE_SET_NOT_SET = 0,
        SCRIPT_WRITE_SET = 2,
        DIRECT_WRITE_SET = 3,
    }

}

export class ScriptWriteSet extends jspb.Message { 
    getExecuteAs(): string;
    setExecuteAs(value: string): ScriptWriteSet;

    hasScript(): boolean;
    clearScript(): void;
    getScript(): ScriptPayload | undefined;
    setScript(value?: ScriptPayload): ScriptWriteSet;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ScriptWriteSet.AsObject;
    static toObject(includeInstance: boolean, msg: ScriptWriteSet): ScriptWriteSet.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ScriptWriteSet, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ScriptWriteSet;
    static deserializeBinaryFromReader(message: ScriptWriteSet, reader: jspb.BinaryReader): ScriptWriteSet;
}

export namespace ScriptWriteSet {
    export type AsObject = {
        executeAs: string,
        script?: ScriptPayload.AsObject,
    }
}

export class DirectWriteSet extends jspb.Message { 
    clearWriteSetChangeList(): void;
    getWriteSetChangeList(): Array<WriteSetChange>;
    setWriteSetChangeList(value: Array<WriteSetChange>): DirectWriteSet;
    addWriteSetChange(value?: WriteSetChange, index?: number): WriteSetChange;
    clearEventsList(): void;
    getEventsList(): Array<Event>;
    setEventsList(value: Array<Event>): DirectWriteSet;
    addEvents(value?: Event, index?: number): Event;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DirectWriteSet.AsObject;
    static toObject(includeInstance: boolean, msg: DirectWriteSet): DirectWriteSet.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DirectWriteSet, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DirectWriteSet;
    static deserializeBinaryFromReader(message: DirectWriteSet, reader: jspb.BinaryReader): DirectWriteSet;
}

export namespace DirectWriteSet {
    export type AsObject = {
        writeSetChangeList: Array<WriteSetChange.AsObject>,
        eventsList: Array<Event.AsObject>,
    }
}

export class WriteSetChange extends jspb.Message { 
    getType(): WriteSetChange.Type;
    setType(value: WriteSetChange.Type): WriteSetChange;

    hasDeleteModule(): boolean;
    clearDeleteModule(): void;
    getDeleteModule(): DeleteModule | undefined;
    setDeleteModule(value?: DeleteModule): WriteSetChange;

    hasDeleteResource(): boolean;
    clearDeleteResource(): void;
    getDeleteResource(): DeleteResource | undefined;
    setDeleteResource(value?: DeleteResource): WriteSetChange;

    hasDeleteTableItem(): boolean;
    clearDeleteTableItem(): void;
    getDeleteTableItem(): DeleteTableItem | undefined;
    setDeleteTableItem(value?: DeleteTableItem): WriteSetChange;

    hasWriteModule(): boolean;
    clearWriteModule(): void;
    getWriteModule(): WriteModule | undefined;
    setWriteModule(value?: WriteModule): WriteSetChange;

    hasWriteResource(): boolean;
    clearWriteResource(): void;
    getWriteResource(): WriteResource | undefined;
    setWriteResource(value?: WriteResource): WriteSetChange;

    hasWriteTableItem(): boolean;
    clearWriteTableItem(): void;
    getWriteTableItem(): WriteTableItem | undefined;
    setWriteTableItem(value?: WriteTableItem): WriteSetChange;

    getChangeCase(): WriteSetChange.ChangeCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteSetChange.AsObject;
    static toObject(includeInstance: boolean, msg: WriteSetChange): WriteSetChange.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteSetChange, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteSetChange;
    static deserializeBinaryFromReader(message: WriteSetChange, reader: jspb.BinaryReader): WriteSetChange;
}

export namespace WriteSetChange {
    export type AsObject = {
        type: WriteSetChange.Type,
        deleteModule?: DeleteModule.AsObject,
        deleteResource?: DeleteResource.AsObject,
        deleteTableItem?: DeleteTableItem.AsObject,
        writeModule?: WriteModule.AsObject,
        writeResource?: WriteResource.AsObject,
        writeTableItem?: WriteTableItem.AsObject,
    }

    export enum Type {
    TYPE_UNSPECIFIED = 0,
    TYPE_DELETE_MODULE = 1,
    TYPE_DELETE_RESOURCE = 2,
    TYPE_DELETE_TABLE_ITEM = 3,
    TYPE_WRITE_MODULE = 4,
    TYPE_WRITE_RESOURCE = 5,
    TYPE_WRITE_TABLE_ITEM = 6,
    }


    export enum ChangeCase {
        CHANGE_NOT_SET = 0,
        DELETE_MODULE = 2,
        DELETE_RESOURCE = 3,
        DELETE_TABLE_ITEM = 4,
        WRITE_MODULE = 5,
        WRITE_RESOURCE = 6,
        WRITE_TABLE_ITEM = 7,
    }

}

export class DeleteModule extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): DeleteModule;
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): DeleteModule;

    hasModule(): boolean;
    clearModule(): void;
    getModule(): MoveModuleId | undefined;
    setModule(value?: MoveModuleId): DeleteModule;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DeleteModule.AsObject;
    static toObject(includeInstance: boolean, msg: DeleteModule): DeleteModule.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DeleteModule, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DeleteModule;
    static deserializeBinaryFromReader(message: DeleteModule, reader: jspb.BinaryReader): DeleteModule;
}

export namespace DeleteModule {
    export type AsObject = {
        address: string,
        stateKeyHash: Uint8Array | string,
        module?: MoveModuleId.AsObject,
    }
}

export class DeleteResource extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): DeleteResource;
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): DeleteResource;

    hasType(): boolean;
    clearType(): void;
    getType(): MoveStructTag | undefined;
    setType(value?: MoveStructTag): DeleteResource;
    getTypeStr(): string;
    setTypeStr(value: string): DeleteResource;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DeleteResource.AsObject;
    static toObject(includeInstance: boolean, msg: DeleteResource): DeleteResource.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DeleteResource, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DeleteResource;
    static deserializeBinaryFromReader(message: DeleteResource, reader: jspb.BinaryReader): DeleteResource;
}

export namespace DeleteResource {
    export type AsObject = {
        address: string,
        stateKeyHash: Uint8Array | string,
        type?: MoveStructTag.AsObject,
        typeStr: string,
    }
}

export class DeleteTableItem extends jspb.Message { 
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): DeleteTableItem;
    getHandle(): string;
    setHandle(value: string): DeleteTableItem;
    getKey(): string;
    setKey(value: string): DeleteTableItem;

    hasData(): boolean;
    clearData(): void;
    getData(): DeleteTableData | undefined;
    setData(value?: DeleteTableData): DeleteTableItem;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DeleteTableItem.AsObject;
    static toObject(includeInstance: boolean, msg: DeleteTableItem): DeleteTableItem.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DeleteTableItem, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DeleteTableItem;
    static deserializeBinaryFromReader(message: DeleteTableItem, reader: jspb.BinaryReader): DeleteTableItem;
}

export namespace DeleteTableItem {
    export type AsObject = {
        stateKeyHash: Uint8Array | string,
        handle: string,
        key: string,
        data?: DeleteTableData.AsObject,
    }
}

export class DeleteTableData extends jspb.Message { 
    getKey(): string;
    setKey(value: string): DeleteTableData;
    getKeyType(): string;
    setKeyType(value: string): DeleteTableData;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DeleteTableData.AsObject;
    static toObject(includeInstance: boolean, msg: DeleteTableData): DeleteTableData.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DeleteTableData, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DeleteTableData;
    static deserializeBinaryFromReader(message: DeleteTableData, reader: jspb.BinaryReader): DeleteTableData;
}

export namespace DeleteTableData {
    export type AsObject = {
        key: string,
        keyType: string,
    }
}

export class WriteModule extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): WriteModule;
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): WriteModule;

    hasData(): boolean;
    clearData(): void;
    getData(): MoveModuleBytecode | undefined;
    setData(value?: MoveModuleBytecode): WriteModule;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteModule.AsObject;
    static toObject(includeInstance: boolean, msg: WriteModule): WriteModule.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteModule, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteModule;
    static deserializeBinaryFromReader(message: WriteModule, reader: jspb.BinaryReader): WriteModule;
}

export namespace WriteModule {
    export type AsObject = {
        address: string,
        stateKeyHash: Uint8Array | string,
        data?: MoveModuleBytecode.AsObject,
    }
}

export class WriteResource extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): WriteResource;
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): WriteResource;

    hasType(): boolean;
    clearType(): void;
    getType(): MoveStructTag | undefined;
    setType(value?: MoveStructTag): WriteResource;
    getTypeStr(): string;
    setTypeStr(value: string): WriteResource;
    getData(): string;
    setData(value: string): WriteResource;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteResource.AsObject;
    static toObject(includeInstance: boolean, msg: WriteResource): WriteResource.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteResource, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteResource;
    static deserializeBinaryFromReader(message: WriteResource, reader: jspb.BinaryReader): WriteResource;
}

export namespace WriteResource {
    export type AsObject = {
        address: string,
        stateKeyHash: Uint8Array | string,
        type?: MoveStructTag.AsObject,
        typeStr: string,
        data: string,
    }
}

export class WriteTableData extends jspb.Message { 
    getKey(): string;
    setKey(value: string): WriteTableData;
    getKeyType(): string;
    setKeyType(value: string): WriteTableData;
    getValue(): string;
    setValue(value: string): WriteTableData;
    getValueType(): string;
    setValueType(value: string): WriteTableData;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteTableData.AsObject;
    static toObject(includeInstance: boolean, msg: WriteTableData): WriteTableData.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteTableData, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteTableData;
    static deserializeBinaryFromReader(message: WriteTableData, reader: jspb.BinaryReader): WriteTableData;
}

export namespace WriteTableData {
    export type AsObject = {
        key: string,
        keyType: string,
        value: string,
        valueType: string,
    }
}

export class WriteTableItem extends jspb.Message { 
    getStateKeyHash(): Uint8Array | string;
    getStateKeyHash_asU8(): Uint8Array;
    getStateKeyHash_asB64(): string;
    setStateKeyHash(value: Uint8Array | string): WriteTableItem;
    getHandle(): string;
    setHandle(value: string): WriteTableItem;
    getKey(): string;
    setKey(value: string): WriteTableItem;

    hasData(): boolean;
    clearData(): void;
    getData(): WriteTableData | undefined;
    setData(value?: WriteTableData): WriteTableItem;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteTableItem.AsObject;
    static toObject(includeInstance: boolean, msg: WriteTableItem): WriteTableItem.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteTableItem, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteTableItem;
    static deserializeBinaryFromReader(message: WriteTableItem, reader: jspb.BinaryReader): WriteTableItem;
}

export namespace WriteTableItem {
    export type AsObject = {
        stateKeyHash: Uint8Array | string,
        handle: string,
        key: string,
        data?: WriteTableData.AsObject,
    }
}

export class TransactionPayload extends jspb.Message { 
    getType(): TransactionPayload.Type;
    setType(value: TransactionPayload.Type): TransactionPayload;

    hasEntryFunctionPayload(): boolean;
    clearEntryFunctionPayload(): void;
    getEntryFunctionPayload(): EntryFunctionPayload | undefined;
    setEntryFunctionPayload(value?: EntryFunctionPayload): TransactionPayload;

    hasScriptPayload(): boolean;
    clearScriptPayload(): void;
    getScriptPayload(): ScriptPayload | undefined;
    setScriptPayload(value?: ScriptPayload): TransactionPayload;

    hasModuleBundlePayload(): boolean;
    clearModuleBundlePayload(): void;
    getModuleBundlePayload(): ModuleBundlePayload | undefined;
    setModuleBundlePayload(value?: ModuleBundlePayload): TransactionPayload;

    hasWriteSetPayload(): boolean;
    clearWriteSetPayload(): void;
    getWriteSetPayload(): WriteSetPayload | undefined;
    setWriteSetPayload(value?: WriteSetPayload): TransactionPayload;

    hasMultisigPayload(): boolean;
    clearMultisigPayload(): void;
    getMultisigPayload(): MultisigPayload | undefined;
    setMultisigPayload(value?: MultisigPayload): TransactionPayload;

    getPayloadCase(): TransactionPayload.PayloadCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): TransactionPayload.AsObject;
    static toObject(includeInstance: boolean, msg: TransactionPayload): TransactionPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: TransactionPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): TransactionPayload;
    static deserializeBinaryFromReader(message: TransactionPayload, reader: jspb.BinaryReader): TransactionPayload;
}

export namespace TransactionPayload {
    export type AsObject = {
        type: TransactionPayload.Type,
        entryFunctionPayload?: EntryFunctionPayload.AsObject,
        scriptPayload?: ScriptPayload.AsObject,
        moduleBundlePayload?: ModuleBundlePayload.AsObject,
        writeSetPayload?: WriteSetPayload.AsObject,
        multisigPayload?: MultisigPayload.AsObject,
    }

    export enum Type {
    TYPE_UNSPECIFIED = 0,
    TYPE_ENTRY_FUNCTION_PAYLOAD = 1,
    TYPE_SCRIPT_PAYLOAD = 2,
    TYPE_MODULE_BUNDLE_PAYLOAD = 3,
    TYPE_WRITE_SET_PAYLOAD = 4,
    TYPE_MULTISIG_PAYLOAD = 5,
    }


    export enum PayloadCase {
        PAYLOAD_NOT_SET = 0,
        ENTRY_FUNCTION_PAYLOAD = 2,
        SCRIPT_PAYLOAD = 3,
        MODULE_BUNDLE_PAYLOAD = 4,
        WRITE_SET_PAYLOAD = 5,
        MULTISIG_PAYLOAD = 6,
    }

}

export class EntryFunctionPayload extends jspb.Message { 

    hasFunction(): boolean;
    clearFunction(): void;
    getFunction(): EntryFunctionId | undefined;
    setFunction(value?: EntryFunctionId): EntryFunctionPayload;
    clearTypeArgumentsList(): void;
    getTypeArgumentsList(): Array<MoveType>;
    setTypeArgumentsList(value: Array<MoveType>): EntryFunctionPayload;
    addTypeArguments(value?: MoveType, index?: number): MoveType;
    clearArgumentsList(): void;
    getArgumentsList(): Array<string>;
    setArgumentsList(value: Array<string>): EntryFunctionPayload;
    addArguments(value: string, index?: number): string;
    getEntryFunctionIdStr(): string;
    setEntryFunctionIdStr(value: string): EntryFunctionPayload;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): EntryFunctionPayload.AsObject;
    static toObject(includeInstance: boolean, msg: EntryFunctionPayload): EntryFunctionPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: EntryFunctionPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): EntryFunctionPayload;
    static deserializeBinaryFromReader(message: EntryFunctionPayload, reader: jspb.BinaryReader): EntryFunctionPayload;
}

export namespace EntryFunctionPayload {
    export type AsObject = {
        pb_function?: EntryFunctionId.AsObject,
        typeArgumentsList: Array<MoveType.AsObject>,
        argumentsList: Array<string>,
        entryFunctionIdStr: string,
    }
}

export class MoveScriptBytecode extends jspb.Message { 
    getBytecode(): Uint8Array | string;
    getBytecode_asU8(): Uint8Array;
    getBytecode_asB64(): string;
    setBytecode(value: Uint8Array | string): MoveScriptBytecode;

    hasAbi(): boolean;
    clearAbi(): void;
    getAbi(): MoveFunction | undefined;
    setAbi(value?: MoveFunction): MoveScriptBytecode;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveScriptBytecode.AsObject;
    static toObject(includeInstance: boolean, msg: MoveScriptBytecode): MoveScriptBytecode.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveScriptBytecode, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveScriptBytecode;
    static deserializeBinaryFromReader(message: MoveScriptBytecode, reader: jspb.BinaryReader): MoveScriptBytecode;
}

export namespace MoveScriptBytecode {
    export type AsObject = {
        bytecode: Uint8Array | string,
        abi?: MoveFunction.AsObject,
    }
}

export class ScriptPayload extends jspb.Message { 

    hasCode(): boolean;
    clearCode(): void;
    getCode(): MoveScriptBytecode | undefined;
    setCode(value?: MoveScriptBytecode): ScriptPayload;
    clearTypeArgumentsList(): void;
    getTypeArgumentsList(): Array<MoveType>;
    setTypeArgumentsList(value: Array<MoveType>): ScriptPayload;
    addTypeArguments(value?: MoveType, index?: number): MoveType;
    clearArgumentsList(): void;
    getArgumentsList(): Array<string>;
    setArgumentsList(value: Array<string>): ScriptPayload;
    addArguments(value: string, index?: number): string;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ScriptPayload.AsObject;
    static toObject(includeInstance: boolean, msg: ScriptPayload): ScriptPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ScriptPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ScriptPayload;
    static deserializeBinaryFromReader(message: ScriptPayload, reader: jspb.BinaryReader): ScriptPayload;
}

export namespace ScriptPayload {
    export type AsObject = {
        code?: MoveScriptBytecode.AsObject,
        typeArgumentsList: Array<MoveType.AsObject>,
        argumentsList: Array<string>,
    }
}

export class MultisigPayload extends jspb.Message { 
    getMultisigAddress(): string;
    setMultisigAddress(value: string): MultisigPayload;

    hasTransactionPayload(): boolean;
    clearTransactionPayload(): void;
    getTransactionPayload(): MultisigTransactionPayload | undefined;
    setTransactionPayload(value?: MultisigTransactionPayload): MultisigPayload;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MultisigPayload.AsObject;
    static toObject(includeInstance: boolean, msg: MultisigPayload): MultisigPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MultisigPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MultisigPayload;
    static deserializeBinaryFromReader(message: MultisigPayload, reader: jspb.BinaryReader): MultisigPayload;
}

export namespace MultisigPayload {
    export type AsObject = {
        multisigAddress: string,
        transactionPayload?: MultisigTransactionPayload.AsObject,
    }
}

export class MultisigTransactionPayload extends jspb.Message { 
    getType(): MultisigTransactionPayload.Type;
    setType(value: MultisigTransactionPayload.Type): MultisigTransactionPayload;

    hasEntryFunctionPayload(): boolean;
    clearEntryFunctionPayload(): void;
    getEntryFunctionPayload(): EntryFunctionPayload | undefined;
    setEntryFunctionPayload(value?: EntryFunctionPayload): MultisigTransactionPayload;

    getPayloadCase(): MultisigTransactionPayload.PayloadCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MultisigTransactionPayload.AsObject;
    static toObject(includeInstance: boolean, msg: MultisigTransactionPayload): MultisigTransactionPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MultisigTransactionPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MultisigTransactionPayload;
    static deserializeBinaryFromReader(message: MultisigTransactionPayload, reader: jspb.BinaryReader): MultisigTransactionPayload;
}

export namespace MultisigTransactionPayload {
    export type AsObject = {
        type: MultisigTransactionPayload.Type,
        entryFunctionPayload?: EntryFunctionPayload.AsObject,
    }

    export enum Type {
    TYPE_UNSPECIFIED = 0,
    TYPE_ENTRY_FUNCTION_PAYLOAD = 1,
    }


    export enum PayloadCase {
        PAYLOAD_NOT_SET = 0,
        ENTRY_FUNCTION_PAYLOAD = 2,
    }

}

export class ModuleBundlePayload extends jspb.Message { 
    clearModulesList(): void;
    getModulesList(): Array<MoveModuleBytecode>;
    setModulesList(value: Array<MoveModuleBytecode>): ModuleBundlePayload;
    addModules(value?: MoveModuleBytecode, index?: number): MoveModuleBytecode;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ModuleBundlePayload.AsObject;
    static toObject(includeInstance: boolean, msg: ModuleBundlePayload): ModuleBundlePayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ModuleBundlePayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ModuleBundlePayload;
    static deserializeBinaryFromReader(message: ModuleBundlePayload, reader: jspb.BinaryReader): ModuleBundlePayload;
}

export namespace ModuleBundlePayload {
    export type AsObject = {
        modulesList: Array<MoveModuleBytecode.AsObject>,
    }
}

export class MoveModuleBytecode extends jspb.Message { 
    getBytecode(): Uint8Array | string;
    getBytecode_asU8(): Uint8Array;
    getBytecode_asB64(): string;
    setBytecode(value: Uint8Array | string): MoveModuleBytecode;

    hasAbi(): boolean;
    clearAbi(): void;
    getAbi(): MoveModule | undefined;
    setAbi(value?: MoveModule): MoveModuleBytecode;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveModuleBytecode.AsObject;
    static toObject(includeInstance: boolean, msg: MoveModuleBytecode): MoveModuleBytecode.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveModuleBytecode, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveModuleBytecode;
    static deserializeBinaryFromReader(message: MoveModuleBytecode, reader: jspb.BinaryReader): MoveModuleBytecode;
}

export namespace MoveModuleBytecode {
    export type AsObject = {
        bytecode: Uint8Array | string,
        abi?: MoveModule.AsObject,
    }
}

export class MoveModule extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): MoveModule;
    getName(): string;
    setName(value: string): MoveModule;
    clearFriendsList(): void;
    getFriendsList(): Array<MoveModuleId>;
    setFriendsList(value: Array<MoveModuleId>): MoveModule;
    addFriends(value?: MoveModuleId, index?: number): MoveModuleId;
    clearExposedFunctionsList(): void;
    getExposedFunctionsList(): Array<MoveFunction>;
    setExposedFunctionsList(value: Array<MoveFunction>): MoveModule;
    addExposedFunctions(value?: MoveFunction, index?: number): MoveFunction;
    clearStructsList(): void;
    getStructsList(): Array<MoveStruct>;
    setStructsList(value: Array<MoveStruct>): MoveModule;
    addStructs(value?: MoveStruct, index?: number): MoveStruct;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveModule.AsObject;
    static toObject(includeInstance: boolean, msg: MoveModule): MoveModule.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveModule, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveModule;
    static deserializeBinaryFromReader(message: MoveModule, reader: jspb.BinaryReader): MoveModule;
}

export namespace MoveModule {
    export type AsObject = {
        address: string,
        name: string,
        friendsList: Array<MoveModuleId.AsObject>,
        exposedFunctionsList: Array<MoveFunction.AsObject>,
        structsList: Array<MoveStruct.AsObject>,
    }
}

export class MoveFunction extends jspb.Message { 
    getName(): string;
    setName(value: string): MoveFunction;
    getVisibility(): MoveFunction.Visibility;
    setVisibility(value: MoveFunction.Visibility): MoveFunction;
    getIsEntry(): boolean;
    setIsEntry(value: boolean): MoveFunction;
    clearGenericTypeParamsList(): void;
    getGenericTypeParamsList(): Array<MoveFunctionGenericTypeParam>;
    setGenericTypeParamsList(value: Array<MoveFunctionGenericTypeParam>): MoveFunction;
    addGenericTypeParams(value?: MoveFunctionGenericTypeParam, index?: number): MoveFunctionGenericTypeParam;
    clearParamsList(): void;
    getParamsList(): Array<MoveType>;
    setParamsList(value: Array<MoveType>): MoveFunction;
    addParams(value?: MoveType, index?: number): MoveType;
    clearReturnList(): void;
    getReturnList(): Array<MoveType>;
    setReturnList(value: Array<MoveType>): MoveFunction;
    addReturn(value?: MoveType, index?: number): MoveType;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveFunction.AsObject;
    static toObject(includeInstance: boolean, msg: MoveFunction): MoveFunction.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveFunction, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveFunction;
    static deserializeBinaryFromReader(message: MoveFunction, reader: jspb.BinaryReader): MoveFunction;
}

export namespace MoveFunction {
    export type AsObject = {
        name: string,
        visibility: MoveFunction.Visibility,
        isEntry: boolean,
        genericTypeParamsList: Array<MoveFunctionGenericTypeParam.AsObject>,
        paramsList: Array<MoveType.AsObject>,
        pb_returnList: Array<MoveType.AsObject>,
    }

    export enum Visibility {
    VISIBILITY_UNSPECIFIED = 0,
    VISIBILITY_PRIVATE = 1,
    VISIBILITY_PUBLIC = 2,
    VISIBILITY_FRIEND = 3,
    }

}

export class MoveStruct extends jspb.Message { 
    getName(): string;
    setName(value: string): MoveStruct;
    getIsNative(): boolean;
    setIsNative(value: boolean): MoveStruct;
    clearAbilitiesList(): void;
    getAbilitiesList(): Array<MoveAbility>;
    setAbilitiesList(value: Array<MoveAbility>): MoveStruct;
    addAbilities(value: MoveAbility, index?: number): MoveAbility;
    clearGenericTypeParamsList(): void;
    getGenericTypeParamsList(): Array<MoveStructGenericTypeParam>;
    setGenericTypeParamsList(value: Array<MoveStructGenericTypeParam>): MoveStruct;
    addGenericTypeParams(value?: MoveStructGenericTypeParam, index?: number): MoveStructGenericTypeParam;
    clearFieldsList(): void;
    getFieldsList(): Array<MoveStructField>;
    setFieldsList(value: Array<MoveStructField>): MoveStruct;
    addFields(value?: MoveStructField, index?: number): MoveStructField;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveStruct.AsObject;
    static toObject(includeInstance: boolean, msg: MoveStruct): MoveStruct.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveStruct, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveStruct;
    static deserializeBinaryFromReader(message: MoveStruct, reader: jspb.BinaryReader): MoveStruct;
}

export namespace MoveStruct {
    export type AsObject = {
        name: string,
        isNative: boolean,
        abilitiesList: Array<MoveAbility>,
        genericTypeParamsList: Array<MoveStructGenericTypeParam.AsObject>,
        fieldsList: Array<MoveStructField.AsObject>,
    }
}

export class MoveStructGenericTypeParam extends jspb.Message { 
    clearConstraintsList(): void;
    getConstraintsList(): Array<MoveAbility>;
    setConstraintsList(value: Array<MoveAbility>): MoveStructGenericTypeParam;
    addConstraints(value: MoveAbility, index?: number): MoveAbility;
    getIsPhantom(): boolean;
    setIsPhantom(value: boolean): MoveStructGenericTypeParam;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveStructGenericTypeParam.AsObject;
    static toObject(includeInstance: boolean, msg: MoveStructGenericTypeParam): MoveStructGenericTypeParam.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveStructGenericTypeParam, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveStructGenericTypeParam;
    static deserializeBinaryFromReader(message: MoveStructGenericTypeParam, reader: jspb.BinaryReader): MoveStructGenericTypeParam;
}

export namespace MoveStructGenericTypeParam {
    export type AsObject = {
        constraintsList: Array<MoveAbility>,
        isPhantom: boolean,
    }
}

export class MoveStructField extends jspb.Message { 
    getName(): string;
    setName(value: string): MoveStructField;

    hasType(): boolean;
    clearType(): void;
    getType(): MoveType | undefined;
    setType(value?: MoveType): MoveStructField;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveStructField.AsObject;
    static toObject(includeInstance: boolean, msg: MoveStructField): MoveStructField.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveStructField, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveStructField;
    static deserializeBinaryFromReader(message: MoveStructField, reader: jspb.BinaryReader): MoveStructField;
}

export namespace MoveStructField {
    export type AsObject = {
        name: string,
        type?: MoveType.AsObject,
    }
}

export class MoveFunctionGenericTypeParam extends jspb.Message { 
    clearConstraintsList(): void;
    getConstraintsList(): Array<MoveAbility>;
    setConstraintsList(value: Array<MoveAbility>): MoveFunctionGenericTypeParam;
    addConstraints(value: MoveAbility, index?: number): MoveAbility;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveFunctionGenericTypeParam.AsObject;
    static toObject(includeInstance: boolean, msg: MoveFunctionGenericTypeParam): MoveFunctionGenericTypeParam.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveFunctionGenericTypeParam, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveFunctionGenericTypeParam;
    static deserializeBinaryFromReader(message: MoveFunctionGenericTypeParam, reader: jspb.BinaryReader): MoveFunctionGenericTypeParam;
}

export namespace MoveFunctionGenericTypeParam {
    export type AsObject = {
        constraintsList: Array<MoveAbility>,
    }
}

export class MoveType extends jspb.Message { 
    getType(): MoveTypes;
    setType(value: MoveTypes): MoveType;

    hasVector(): boolean;
    clearVector(): void;
    getVector(): MoveType | undefined;
    setVector(value?: MoveType): MoveType;

    hasStruct(): boolean;
    clearStruct(): void;
    getStruct(): MoveStructTag | undefined;
    setStruct(value?: MoveStructTag): MoveType;

    hasGenericTypeParamIndex(): boolean;
    clearGenericTypeParamIndex(): void;
    getGenericTypeParamIndex(): number;
    setGenericTypeParamIndex(value: number): MoveType;

    hasReference(): boolean;
    clearReference(): void;
    getReference(): MoveType.ReferenceType | undefined;
    setReference(value?: MoveType.ReferenceType): MoveType;

    hasUnparsable(): boolean;
    clearUnparsable(): void;
    getUnparsable(): string;
    setUnparsable(value: string): MoveType;

    getContentCase(): MoveType.ContentCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveType.AsObject;
    static toObject(includeInstance: boolean, msg: MoveType): MoveType.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveType, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveType;
    static deserializeBinaryFromReader(message: MoveType, reader: jspb.BinaryReader): MoveType;
}

export namespace MoveType {
    export type AsObject = {
        type: MoveTypes,
        vector?: MoveType.AsObject,
        struct?: MoveStructTag.AsObject,
        genericTypeParamIndex: number,
        reference?: MoveType.ReferenceType.AsObject,
        unparsable: string,
    }


    export class ReferenceType extends jspb.Message { 
        getMutable(): boolean;
        setMutable(value: boolean): ReferenceType;

        hasTo(): boolean;
        clearTo(): void;
        getTo(): MoveType | undefined;
        setTo(value?: MoveType): ReferenceType;

        serializeBinary(): Uint8Array;
        toObject(includeInstance?: boolean): ReferenceType.AsObject;
        static toObject(includeInstance: boolean, msg: ReferenceType): ReferenceType.AsObject;
        static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
        static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
        static serializeBinaryToWriter(message: ReferenceType, writer: jspb.BinaryWriter): void;
        static deserializeBinary(bytes: Uint8Array): ReferenceType;
        static deserializeBinaryFromReader(message: ReferenceType, reader: jspb.BinaryReader): ReferenceType;
    }

    export namespace ReferenceType {
        export type AsObject = {
            mutable: boolean,
            to?: MoveType.AsObject,
        }
    }


    export enum ContentCase {
        CONTENT_NOT_SET = 0,
        VECTOR = 3,
        STRUCT = 4,
        GENERIC_TYPE_PARAM_INDEX = 5,
        REFERENCE = 6,
        UNPARSABLE = 7,
    }

}

export class WriteSetPayload extends jspb.Message { 

    hasWriteSet(): boolean;
    clearWriteSet(): void;
    getWriteSet(): WriteSet | undefined;
    setWriteSet(value?: WriteSet): WriteSetPayload;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WriteSetPayload.AsObject;
    static toObject(includeInstance: boolean, msg: WriteSetPayload): WriteSetPayload.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WriteSetPayload, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WriteSetPayload;
    static deserializeBinaryFromReader(message: WriteSetPayload, reader: jspb.BinaryReader): WriteSetPayload;
}

export namespace WriteSetPayload {
    export type AsObject = {
        writeSet?: WriteSet.AsObject,
    }
}

export class EntryFunctionId extends jspb.Message { 

    hasModule(): boolean;
    clearModule(): void;
    getModule(): MoveModuleId | undefined;
    setModule(value?: MoveModuleId): EntryFunctionId;
    getName(): string;
    setName(value: string): EntryFunctionId;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): EntryFunctionId.AsObject;
    static toObject(includeInstance: boolean, msg: EntryFunctionId): EntryFunctionId.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: EntryFunctionId, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): EntryFunctionId;
    static deserializeBinaryFromReader(message: EntryFunctionId, reader: jspb.BinaryReader): EntryFunctionId;
}

export namespace EntryFunctionId {
    export type AsObject = {
        module?: MoveModuleId.AsObject,
        name: string,
    }
}

export class MoveModuleId extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): MoveModuleId;
    getName(): string;
    setName(value: string): MoveModuleId;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveModuleId.AsObject;
    static toObject(includeInstance: boolean, msg: MoveModuleId): MoveModuleId.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveModuleId, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveModuleId;
    static deserializeBinaryFromReader(message: MoveModuleId, reader: jspb.BinaryReader): MoveModuleId;
}

export namespace MoveModuleId {
    export type AsObject = {
        address: string,
        name: string,
    }
}

export class MoveStructTag extends jspb.Message { 
    getAddress(): string;
    setAddress(value: string): MoveStructTag;
    getModule(): string;
    setModule(value: string): MoveStructTag;
    getName(): string;
    setName(value: string): MoveStructTag;
    clearGenericTypeParamsList(): void;
    getGenericTypeParamsList(): Array<MoveType>;
    setGenericTypeParamsList(value: Array<MoveType>): MoveStructTag;
    addGenericTypeParams(value?: MoveType, index?: number): MoveType;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MoveStructTag.AsObject;
    static toObject(includeInstance: boolean, msg: MoveStructTag): MoveStructTag.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MoveStructTag, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MoveStructTag;
    static deserializeBinaryFromReader(message: MoveStructTag, reader: jspb.BinaryReader): MoveStructTag;
}

export namespace MoveStructTag {
    export type AsObject = {
        address: string,
        module: string,
        name: string,
        genericTypeParamsList: Array<MoveType.AsObject>,
    }
}

export class Signature extends jspb.Message { 
    getType(): Signature.Type;
    setType(value: Signature.Type): Signature;

    hasEd25519(): boolean;
    clearEd25519(): void;
    getEd25519(): Ed25519Signature | undefined;
    setEd25519(value?: Ed25519Signature): Signature;

    hasMultiEd25519(): boolean;
    clearMultiEd25519(): void;
    getMultiEd25519(): MultiEd25519Signature | undefined;
    setMultiEd25519(value?: MultiEd25519Signature): Signature;

    hasMultiAgent(): boolean;
    clearMultiAgent(): void;
    getMultiAgent(): MultiAgentSignature | undefined;
    setMultiAgent(value?: MultiAgentSignature): Signature;

    getSignatureCase(): Signature.SignatureCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Signature.AsObject;
    static toObject(includeInstance: boolean, msg: Signature): Signature.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Signature, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Signature;
    static deserializeBinaryFromReader(message: Signature, reader: jspb.BinaryReader): Signature;
}

export namespace Signature {
    export type AsObject = {
        type: Signature.Type,
        ed25519?: Ed25519Signature.AsObject,
        multiEd25519?: MultiEd25519Signature.AsObject,
        multiAgent?: MultiAgentSignature.AsObject,
    }

    export enum Type {
    TYPE_UNSPECIFIED = 0,
    TYPE_ED25519 = 1,
    TYPE_MULTI_ED25519 = 2,
    TYPE_MULTI_AGENT = 3,
    }


    export enum SignatureCase {
        SIGNATURE_NOT_SET = 0,
        ED25519 = 2,
        MULTI_ED25519 = 3,
        MULTI_AGENT = 4,
    }

}

export class Ed25519Signature extends jspb.Message { 
    getPublicKey(): Uint8Array | string;
    getPublicKey_asU8(): Uint8Array;
    getPublicKey_asB64(): string;
    setPublicKey(value: Uint8Array | string): Ed25519Signature;
    getSignature(): Uint8Array | string;
    getSignature_asU8(): Uint8Array;
    getSignature_asB64(): string;
    setSignature(value: Uint8Array | string): Ed25519Signature;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Ed25519Signature.AsObject;
    static toObject(includeInstance: boolean, msg: Ed25519Signature): Ed25519Signature.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Ed25519Signature, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Ed25519Signature;
    static deserializeBinaryFromReader(message: Ed25519Signature, reader: jspb.BinaryReader): Ed25519Signature;
}

export namespace Ed25519Signature {
    export type AsObject = {
        publicKey: Uint8Array | string,
        signature: Uint8Array | string,
    }
}

export class MultiEd25519Signature extends jspb.Message { 
    clearPublicKeysList(): void;
    getPublicKeysList(): Array<Uint8Array | string>;
    getPublicKeysList_asU8(): Array<Uint8Array>;
    getPublicKeysList_asB64(): Array<string>;
    setPublicKeysList(value: Array<Uint8Array | string>): MultiEd25519Signature;
    addPublicKeys(value: Uint8Array | string, index?: number): Uint8Array | string;
    clearSignaturesList(): void;
    getSignaturesList(): Array<Uint8Array | string>;
    getSignaturesList_asU8(): Array<Uint8Array>;
    getSignaturesList_asB64(): Array<string>;
    setSignaturesList(value: Array<Uint8Array | string>): MultiEd25519Signature;
    addSignatures(value: Uint8Array | string, index?: number): Uint8Array | string;
    getThreshold(): number;
    setThreshold(value: number): MultiEd25519Signature;
    clearPublicKeyIndicesList(): void;
    getPublicKeyIndicesList(): Array<number>;
    setPublicKeyIndicesList(value: Array<number>): MultiEd25519Signature;
    addPublicKeyIndices(value: number, index?: number): number;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MultiEd25519Signature.AsObject;
    static toObject(includeInstance: boolean, msg: MultiEd25519Signature): MultiEd25519Signature.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MultiEd25519Signature, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MultiEd25519Signature;
    static deserializeBinaryFromReader(message: MultiEd25519Signature, reader: jspb.BinaryReader): MultiEd25519Signature;
}

export namespace MultiEd25519Signature {
    export type AsObject = {
        publicKeysList: Array<Uint8Array | string>,
        signaturesList: Array<Uint8Array | string>,
        threshold: number,
        publicKeyIndicesList: Array<number>,
    }
}

export class MultiAgentSignature extends jspb.Message { 

    hasSender(): boolean;
    clearSender(): void;
    getSender(): AccountSignature | undefined;
    setSender(value?: AccountSignature): MultiAgentSignature;
    clearSecondarySignerAddressesList(): void;
    getSecondarySignerAddressesList(): Array<string>;
    setSecondarySignerAddressesList(value: Array<string>): MultiAgentSignature;
    addSecondarySignerAddresses(value: string, index?: number): string;
    clearSecondarySignersList(): void;
    getSecondarySignersList(): Array<AccountSignature>;
    setSecondarySignersList(value: Array<AccountSignature>): MultiAgentSignature;
    addSecondarySigners(value?: AccountSignature, index?: number): AccountSignature;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): MultiAgentSignature.AsObject;
    static toObject(includeInstance: boolean, msg: MultiAgentSignature): MultiAgentSignature.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: MultiAgentSignature, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): MultiAgentSignature;
    static deserializeBinaryFromReader(message: MultiAgentSignature, reader: jspb.BinaryReader): MultiAgentSignature;
}

export namespace MultiAgentSignature {
    export type AsObject = {
        sender?: AccountSignature.AsObject,
        secondarySignerAddressesList: Array<string>,
        secondarySignersList: Array<AccountSignature.AsObject>,
    }
}

export class AccountSignature extends jspb.Message { 
    getType(): AccountSignature.Type;
    setType(value: AccountSignature.Type): AccountSignature;

    hasEd25519(): boolean;
    clearEd25519(): void;
    getEd25519(): Ed25519Signature | undefined;
    setEd25519(value?: Ed25519Signature): AccountSignature;

    hasMultiEd25519(): boolean;
    clearMultiEd25519(): void;
    getMultiEd25519(): MultiEd25519Signature | undefined;
    setMultiEd25519(value?: MultiEd25519Signature): AccountSignature;

    getSignatureCase(): AccountSignature.SignatureCase;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): AccountSignature.AsObject;
    static toObject(includeInstance: boolean, msg: AccountSignature): AccountSignature.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: AccountSignature, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): AccountSignature;
    static deserializeBinaryFromReader(message: AccountSignature, reader: jspb.BinaryReader): AccountSignature;
}

export namespace AccountSignature {
    export type AsObject = {
        type: AccountSignature.Type,
        ed25519?: Ed25519Signature.AsObject,
        multiEd25519?: MultiEd25519Signature.AsObject,
    }

    export enum Type {
    TYPE_UNSPECIFIED = 0,
    TYPE_ED25519 = 1,
    TYPE_MULTI_ED25519 = 2,
    }


    export enum SignatureCase {
        SIGNATURE_NOT_SET = 0,
        ED25519 = 2,
        MULTI_ED25519 = 3,
    }

}

export enum MoveTypes {
    MOVE_TYPES_UNSPECIFIED = 0,
    MOVE_TYPES_BOOL = 1,
    MOVE_TYPES_U8 = 2,
    MOVE_TYPES_U16 = 12,
    MOVE_TYPES_U32 = 13,
    MOVE_TYPES_U64 = 3,
    MOVE_TYPES_U128 = 4,
    MOVE_TYPES_U256 = 14,
    MOVE_TYPES_ADDRESS = 5,
    MOVE_TYPES_SIGNER = 6,
    MOVE_TYPES_VECTOR = 7,
    MOVE_TYPES_STRUCT = 8,
    MOVE_TYPES_GENERIC_TYPE_PARAM = 9,
    MOVE_TYPES_REFERENCE = 10,
    MOVE_TYPES_UNPARSABLE = 11,
}

export enum MoveAbility {
    MOVE_ABILITY_UNSPECIFIED = 0,
    MOVE_ABILITY_COPY = 1,
    MOVE_ABILITY_DROP = 2,
    MOVE_ABILITY_STORE = 3,
    MOVE_ABILITY_KEY = 4,
}
