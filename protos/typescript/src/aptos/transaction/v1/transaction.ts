/* eslint-disable */
import Long from "long";
import _m0 from "protobufjs/minimal";
import { Timestamp } from "../../util/timestamp/timestamp";

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
  /** MOVE_TYPES_VECTOR - `{ items: Box<MoveType> }`, */
  MOVE_TYPES_VECTOR = 7,
  /** MOVE_TYPES_STRUCT - `(MoveStructTag)`, */
  MOVE_TYPES_STRUCT = 8,
  /** MOVE_TYPES_GENERIC_TYPE_PARAM - `{ index: u16 }``, */
  MOVE_TYPES_GENERIC_TYPE_PARAM = 9,
  /** MOVE_TYPES_REFERENCE - `{ mutable: bool, to: Box<MoveType> }`, */
  MOVE_TYPES_REFERENCE = 10,
  /** MOVE_TYPES_UNPARSABLE - `(String)`, */
  MOVE_TYPES_UNPARSABLE = 11,
  UNRECOGNIZED = -1,
}

export function moveTypesFromJSON(object: any): MoveTypes {
  switch (object) {
    case 0:
    case "MOVE_TYPES_UNSPECIFIED":
      return MoveTypes.MOVE_TYPES_UNSPECIFIED;
    case 1:
    case "MOVE_TYPES_BOOL":
      return MoveTypes.MOVE_TYPES_BOOL;
    case 2:
    case "MOVE_TYPES_U8":
      return MoveTypes.MOVE_TYPES_U8;
    case 12:
    case "MOVE_TYPES_U16":
      return MoveTypes.MOVE_TYPES_U16;
    case 13:
    case "MOVE_TYPES_U32":
      return MoveTypes.MOVE_TYPES_U32;
    case 3:
    case "MOVE_TYPES_U64":
      return MoveTypes.MOVE_TYPES_U64;
    case 4:
    case "MOVE_TYPES_U128":
      return MoveTypes.MOVE_TYPES_U128;
    case 14:
    case "MOVE_TYPES_U256":
      return MoveTypes.MOVE_TYPES_U256;
    case 5:
    case "MOVE_TYPES_ADDRESS":
      return MoveTypes.MOVE_TYPES_ADDRESS;
    case 6:
    case "MOVE_TYPES_SIGNER":
      return MoveTypes.MOVE_TYPES_SIGNER;
    case 7:
    case "MOVE_TYPES_VECTOR":
      return MoveTypes.MOVE_TYPES_VECTOR;
    case 8:
    case "MOVE_TYPES_STRUCT":
      return MoveTypes.MOVE_TYPES_STRUCT;
    case 9:
    case "MOVE_TYPES_GENERIC_TYPE_PARAM":
      return MoveTypes.MOVE_TYPES_GENERIC_TYPE_PARAM;
    case 10:
    case "MOVE_TYPES_REFERENCE":
      return MoveTypes.MOVE_TYPES_REFERENCE;
    case 11:
    case "MOVE_TYPES_UNPARSABLE":
      return MoveTypes.MOVE_TYPES_UNPARSABLE;
    case -1:
    case "UNRECOGNIZED":
    default:
      return MoveTypes.UNRECOGNIZED;
  }
}

export function moveTypesToJSON(object: MoveTypes): string {
  switch (object) {
    case MoveTypes.MOVE_TYPES_UNSPECIFIED:
      return "MOVE_TYPES_UNSPECIFIED";
    case MoveTypes.MOVE_TYPES_BOOL:
      return "MOVE_TYPES_BOOL";
    case MoveTypes.MOVE_TYPES_U8:
      return "MOVE_TYPES_U8";
    case MoveTypes.MOVE_TYPES_U16:
      return "MOVE_TYPES_U16";
    case MoveTypes.MOVE_TYPES_U32:
      return "MOVE_TYPES_U32";
    case MoveTypes.MOVE_TYPES_U64:
      return "MOVE_TYPES_U64";
    case MoveTypes.MOVE_TYPES_U128:
      return "MOVE_TYPES_U128";
    case MoveTypes.MOVE_TYPES_U256:
      return "MOVE_TYPES_U256";
    case MoveTypes.MOVE_TYPES_ADDRESS:
      return "MOVE_TYPES_ADDRESS";
    case MoveTypes.MOVE_TYPES_SIGNER:
      return "MOVE_TYPES_SIGNER";
    case MoveTypes.MOVE_TYPES_VECTOR:
      return "MOVE_TYPES_VECTOR";
    case MoveTypes.MOVE_TYPES_STRUCT:
      return "MOVE_TYPES_STRUCT";
    case MoveTypes.MOVE_TYPES_GENERIC_TYPE_PARAM:
      return "MOVE_TYPES_GENERIC_TYPE_PARAM";
    case MoveTypes.MOVE_TYPES_REFERENCE:
      return "MOVE_TYPES_REFERENCE";
    case MoveTypes.MOVE_TYPES_UNPARSABLE:
      return "MOVE_TYPES_UNPARSABLE";
    case MoveTypes.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum MoveAbility {
  MOVE_ABILITY_UNSPECIFIED = 0,
  MOVE_ABILITY_COPY = 1,
  MOVE_ABILITY_DROP = 2,
  MOVE_ABILITY_STORE = 3,
  MOVE_ABILITY_KEY = 4,
  UNRECOGNIZED = -1,
}

export function moveAbilityFromJSON(object: any): MoveAbility {
  switch (object) {
    case 0:
    case "MOVE_ABILITY_UNSPECIFIED":
      return MoveAbility.MOVE_ABILITY_UNSPECIFIED;
    case 1:
    case "MOVE_ABILITY_COPY":
      return MoveAbility.MOVE_ABILITY_COPY;
    case 2:
    case "MOVE_ABILITY_DROP":
      return MoveAbility.MOVE_ABILITY_DROP;
    case 3:
    case "MOVE_ABILITY_STORE":
      return MoveAbility.MOVE_ABILITY_STORE;
    case 4:
    case "MOVE_ABILITY_KEY":
      return MoveAbility.MOVE_ABILITY_KEY;
    case -1:
    case "UNRECOGNIZED":
    default:
      return MoveAbility.UNRECOGNIZED;
  }
}

export function moveAbilityToJSON(object: MoveAbility): string {
  switch (object) {
    case MoveAbility.MOVE_ABILITY_UNSPECIFIED:
      return "MOVE_ABILITY_UNSPECIFIED";
    case MoveAbility.MOVE_ABILITY_COPY:
      return "MOVE_ABILITY_COPY";
    case MoveAbility.MOVE_ABILITY_DROP:
      return "MOVE_ABILITY_DROP";
    case MoveAbility.MOVE_ABILITY_STORE:
      return "MOVE_ABILITY_STORE";
    case MoveAbility.MOVE_ABILITY_KEY:
      return "MOVE_ABILITY_KEY";
    case MoveAbility.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

/**
 * A block on Aptos holds transactions in chronological order (ordered by a transactions monotonically increasing `version` field)
 * All blocks start with a `BlockMetadataTransaction`, and are followed by zero or more transactions.
 * The next `BlockMetadataTransaction` denotes the end of the current block, and the start of the next one.
 *
 * The Block `height` is a strictly monotonically increasing count of the number of blocks,
 * and there will never be a gap in the numbers. It is also a unique identifier: there will never be two blocks with
 * the same `height`.
 *
 * The Genesis Transaction (version 0) is contained within the first block, which has a height of `0`
 */
export interface Block {
  /**
   * Timestamp represents the timestamp of the `BlockMetadataTransaction` (or `GenesisTransaction` for the genesis block)
   * and every transaction in the `transactions` will have the same `timestamp` as the block.
   */
  timestamp?:
    | Timestamp
    | undefined;
  /** Height represents the block number and ultimately, is the count of `BlockMetadataTransaction` that happened on the chain. */
  height?:
    | bigint
    | undefined;
  /**
   * Transactions holds all transactions that happened in the Block, which is transactions that happened starting with (and including)
   * a `BlockMetadataTransaction`, and every other transaction up to (but excluding) the next `BlockMetadataTransaction`.
   */
  transactions?:
    | Transaction[]
    | undefined;
  /** Chain ID informs us which chain we're trying to index, this is important to ensure that we're not mixing chains within a single pipeline. */
  chainId?: number | undefined;
}

/**
 * Transaction as it happened on the chain, there are 4 types of transactions:
 * - User Transaction: a user initiated transaction to interact with the chain
 * - Block Metadata Transaction: transactions generated by the chain to group together transactions forming a "block"
 * - Block Epilogue / State Checkpoint Transaction: transactions generated by the chain to end the group transactions forming a bloc
 * - Genesis Transaction: the first transaction of the chain, with all core contract and validator information baked in
 */
export interface Transaction {
  timestamp?: Timestamp | undefined;
  version?: bigint | undefined;
  info?: TransactionInfo | undefined;
  epoch?: bigint | undefined;
  blockHeight?: bigint | undefined;
  type?: Transaction_TransactionType | undefined;
  blockMetadata?: BlockMetadataTransaction | undefined;
  genesis?: GenesisTransaction | undefined;
  stateCheckpoint?: StateCheckpointTransaction | undefined;
  user?:
    | UserTransaction
    | undefined;
  /** value 11-19 skipped for no reason */
  validator?:
    | ValidatorTransaction
    | undefined;
  /** value 22 is used up below (all Transaction fields have to have different index), so going to 23 */
  blockEpilogue?: BlockEpilogueTransaction | undefined;
  sizeInfo?: TransactionSizeInfo | undefined;
}

export enum Transaction_TransactionType {
  TRANSACTION_TYPE_UNSPECIFIED = 0,
  TRANSACTION_TYPE_GENESIS = 1,
  TRANSACTION_TYPE_BLOCK_METADATA = 2,
  TRANSACTION_TYPE_STATE_CHECKPOINT = 3,
  TRANSACTION_TYPE_USER = 4,
  /** TRANSACTION_TYPE_VALIDATOR - values 5-19 skipped for no reason */
  TRANSACTION_TYPE_VALIDATOR = 20,
  TRANSACTION_TYPE_BLOCK_EPILOGUE = 21,
  UNRECOGNIZED = -1,
}

export function transaction_TransactionTypeFromJSON(object: any): Transaction_TransactionType {
  switch (object) {
    case 0:
    case "TRANSACTION_TYPE_UNSPECIFIED":
      return Transaction_TransactionType.TRANSACTION_TYPE_UNSPECIFIED;
    case 1:
    case "TRANSACTION_TYPE_GENESIS":
      return Transaction_TransactionType.TRANSACTION_TYPE_GENESIS;
    case 2:
    case "TRANSACTION_TYPE_BLOCK_METADATA":
      return Transaction_TransactionType.TRANSACTION_TYPE_BLOCK_METADATA;
    case 3:
    case "TRANSACTION_TYPE_STATE_CHECKPOINT":
      return Transaction_TransactionType.TRANSACTION_TYPE_STATE_CHECKPOINT;
    case 4:
    case "TRANSACTION_TYPE_USER":
      return Transaction_TransactionType.TRANSACTION_TYPE_USER;
    case 20:
    case "TRANSACTION_TYPE_VALIDATOR":
      return Transaction_TransactionType.TRANSACTION_TYPE_VALIDATOR;
    case 21:
    case "TRANSACTION_TYPE_BLOCK_EPILOGUE":
      return Transaction_TransactionType.TRANSACTION_TYPE_BLOCK_EPILOGUE;
    case -1:
    case "UNRECOGNIZED":
    default:
      return Transaction_TransactionType.UNRECOGNIZED;
  }
}

export function transaction_TransactionTypeToJSON(object: Transaction_TransactionType): string {
  switch (object) {
    case Transaction_TransactionType.TRANSACTION_TYPE_UNSPECIFIED:
      return "TRANSACTION_TYPE_UNSPECIFIED";
    case Transaction_TransactionType.TRANSACTION_TYPE_GENESIS:
      return "TRANSACTION_TYPE_GENESIS";
    case Transaction_TransactionType.TRANSACTION_TYPE_BLOCK_METADATA:
      return "TRANSACTION_TYPE_BLOCK_METADATA";
    case Transaction_TransactionType.TRANSACTION_TYPE_STATE_CHECKPOINT:
      return "TRANSACTION_TYPE_STATE_CHECKPOINT";
    case Transaction_TransactionType.TRANSACTION_TYPE_USER:
      return "TRANSACTION_TYPE_USER";
    case Transaction_TransactionType.TRANSACTION_TYPE_VALIDATOR:
      return "TRANSACTION_TYPE_VALIDATOR";
    case Transaction_TransactionType.TRANSACTION_TYPE_BLOCK_EPILOGUE:
      return "TRANSACTION_TYPE_BLOCK_EPILOGUE";
    case Transaction_TransactionType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

/** Transaction types. */
export interface BlockMetadataTransaction {
  id?: string | undefined;
  round?: bigint | undefined;
  events?: Event[] | undefined;
  previousBlockVotesBitvec?: Uint8Array | undefined;
  proposer?: string | undefined;
  failedProposerIndices?: number[] | undefined;
}

export interface GenesisTransaction {
  payload?: WriteSet | undefined;
  events?: Event[] | undefined;
}

export interface StateCheckpointTransaction {
}

export interface ValidatorTransaction {
  observedJwkUpdate?: ValidatorTransaction_ObservedJwkUpdate | undefined;
  dkgUpdate?: ValidatorTransaction_DkgUpdate | undefined;
  events?: Event[] | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate {
  quorumCertifiedUpdate?: ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
  issuer?: string | undefined;
  version?: bigint | undefined;
  jwks?: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK[] | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
  unsupportedJwk?: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK | undefined;
  rsa?: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
  kid?: string | undefined;
  kty?: string | undefined;
  alg?: string | undefined;
  e?: string | undefined;
  n?: string | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
  id?: Uint8Array | undefined;
  payload?: Uint8Array | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
  signerIndices?:
    | bigint[]
    | undefined;
  /** HexToBytes. */
  sig?: Uint8Array | undefined;
}

export interface ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
  update?: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs | undefined;
  multiSig?: ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature | undefined;
}

export interface ValidatorTransaction_DkgUpdate {
  dkgTranscript?: ValidatorTransaction_DkgUpdate_DkgTranscript | undefined;
}

export interface ValidatorTransaction_DkgUpdate_DkgTranscript {
  epoch?: bigint | undefined;
  author?: string | undefined;
  payload?: Uint8Array | undefined;
}

export interface BlockEpilogueTransaction {
  blockEndInfo?: BlockEndInfo | undefined;
}

export interface BlockEndInfo {
  blockGasLimitReached?: boolean | undefined;
  blockOutputLimitReached?: boolean | undefined;
  blockEffectiveBlockGasUnits?: bigint | undefined;
  blockApproxOutputSize?: bigint | undefined;
}

export interface UserTransaction {
  request?: UserTransactionRequest | undefined;
  events?: Event[] | undefined;
}

export interface Event {
  key?: EventKey | undefined;
  sequenceNumber?: bigint | undefined;
  type?: MoveType | undefined;
  typeStr?: string | undefined;
  data?: string | undefined;
}

export interface TransactionInfo {
  hash?: Uint8Array | undefined;
  stateChangeHash?: Uint8Array | undefined;
  eventRootHash?: Uint8Array | undefined;
  stateCheckpointHash?: Uint8Array | undefined;
  gasUsed?: bigint | undefined;
  success?: boolean | undefined;
  vmStatus?: string | undefined;
  accumulatorRootHash?: Uint8Array | undefined;
  changes?: WriteSetChange[] | undefined;
}

export interface EventKey {
  creationNumber?: bigint | undefined;
  accountAddress?: string | undefined;
}

export interface UserTransactionRequest {
  sender?: string | undefined;
  sequenceNumber?: bigint | undefined;
  maxGasAmount?: bigint | undefined;
  gasUnitPrice?: bigint | undefined;
  expirationTimestampSecs?: Timestamp | undefined;
  payload?: TransactionPayload | undefined;
  signature?: Signature | undefined;
}

export interface WriteSet {
  writeSetType?: WriteSet_WriteSetType | undefined;
  scriptWriteSet?: ScriptWriteSet | undefined;
  directWriteSet?: DirectWriteSet | undefined;
}

export enum WriteSet_WriteSetType {
  WRITE_SET_TYPE_UNSPECIFIED = 0,
  WRITE_SET_TYPE_SCRIPT_WRITE_SET = 1,
  WRITE_SET_TYPE_DIRECT_WRITE_SET = 2,
  UNRECOGNIZED = -1,
}

export function writeSet_WriteSetTypeFromJSON(object: any): WriteSet_WriteSetType {
  switch (object) {
    case 0:
    case "WRITE_SET_TYPE_UNSPECIFIED":
      return WriteSet_WriteSetType.WRITE_SET_TYPE_UNSPECIFIED;
    case 1:
    case "WRITE_SET_TYPE_SCRIPT_WRITE_SET":
      return WriteSet_WriteSetType.WRITE_SET_TYPE_SCRIPT_WRITE_SET;
    case 2:
    case "WRITE_SET_TYPE_DIRECT_WRITE_SET":
      return WriteSet_WriteSetType.WRITE_SET_TYPE_DIRECT_WRITE_SET;
    case -1:
    case "UNRECOGNIZED":
    default:
      return WriteSet_WriteSetType.UNRECOGNIZED;
  }
}

export function writeSet_WriteSetTypeToJSON(object: WriteSet_WriteSetType): string {
  switch (object) {
    case WriteSet_WriteSetType.WRITE_SET_TYPE_UNSPECIFIED:
      return "WRITE_SET_TYPE_UNSPECIFIED";
    case WriteSet_WriteSetType.WRITE_SET_TYPE_SCRIPT_WRITE_SET:
      return "WRITE_SET_TYPE_SCRIPT_WRITE_SET";
    case WriteSet_WriteSetType.WRITE_SET_TYPE_DIRECT_WRITE_SET:
      return "WRITE_SET_TYPE_DIRECT_WRITE_SET";
    case WriteSet_WriteSetType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface ScriptWriteSet {
  executeAs?: string | undefined;
  script?: ScriptPayload | undefined;
}

export interface DirectWriteSet {
  writeSetChange?: WriteSetChange[] | undefined;
  events?: Event[] | undefined;
}

export interface WriteSetChange {
  type?: WriteSetChange_Type | undefined;
  deleteModule?: DeleteModule | undefined;
  deleteResource?: DeleteResource | undefined;
  deleteTableItem?: DeleteTableItem | undefined;
  writeModule?: WriteModule | undefined;
  writeResource?: WriteResource | undefined;
  writeTableItem?: WriteTableItem | undefined;
}

export enum WriteSetChange_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_DELETE_MODULE = 1,
  TYPE_DELETE_RESOURCE = 2,
  TYPE_DELETE_TABLE_ITEM = 3,
  TYPE_WRITE_MODULE = 4,
  TYPE_WRITE_RESOURCE = 5,
  TYPE_WRITE_TABLE_ITEM = 6,
  UNRECOGNIZED = -1,
}

export function writeSetChange_TypeFromJSON(object: any): WriteSetChange_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return WriteSetChange_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_DELETE_MODULE":
      return WriteSetChange_Type.TYPE_DELETE_MODULE;
    case 2:
    case "TYPE_DELETE_RESOURCE":
      return WriteSetChange_Type.TYPE_DELETE_RESOURCE;
    case 3:
    case "TYPE_DELETE_TABLE_ITEM":
      return WriteSetChange_Type.TYPE_DELETE_TABLE_ITEM;
    case 4:
    case "TYPE_WRITE_MODULE":
      return WriteSetChange_Type.TYPE_WRITE_MODULE;
    case 5:
    case "TYPE_WRITE_RESOURCE":
      return WriteSetChange_Type.TYPE_WRITE_RESOURCE;
    case 6:
    case "TYPE_WRITE_TABLE_ITEM":
      return WriteSetChange_Type.TYPE_WRITE_TABLE_ITEM;
    case -1:
    case "UNRECOGNIZED":
    default:
      return WriteSetChange_Type.UNRECOGNIZED;
  }
}

export function writeSetChange_TypeToJSON(object: WriteSetChange_Type): string {
  switch (object) {
    case WriteSetChange_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case WriteSetChange_Type.TYPE_DELETE_MODULE:
      return "TYPE_DELETE_MODULE";
    case WriteSetChange_Type.TYPE_DELETE_RESOURCE:
      return "TYPE_DELETE_RESOURCE";
    case WriteSetChange_Type.TYPE_DELETE_TABLE_ITEM:
      return "TYPE_DELETE_TABLE_ITEM";
    case WriteSetChange_Type.TYPE_WRITE_MODULE:
      return "TYPE_WRITE_MODULE";
    case WriteSetChange_Type.TYPE_WRITE_RESOURCE:
      return "TYPE_WRITE_RESOURCE";
    case WriteSetChange_Type.TYPE_WRITE_TABLE_ITEM:
      return "TYPE_WRITE_TABLE_ITEM";
    case WriteSetChange_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface DeleteModule {
  address?: string | undefined;
  stateKeyHash?: Uint8Array | undefined;
  module?: MoveModuleId | undefined;
}

export interface DeleteResource {
  address?: string | undefined;
  stateKeyHash?: Uint8Array | undefined;
  type?: MoveStructTag | undefined;
  typeStr?: string | undefined;
}

export interface DeleteTableItem {
  stateKeyHash?: Uint8Array | undefined;
  handle?: string | undefined;
  key?: string | undefined;
  data?: DeleteTableData | undefined;
}

export interface DeleteTableData {
  key?: string | undefined;
  keyType?: string | undefined;
}

export interface WriteModule {
  address?: string | undefined;
  stateKeyHash?: Uint8Array | undefined;
  data?: MoveModuleBytecode | undefined;
}

export interface WriteResource {
  address?: string | undefined;
  stateKeyHash?: Uint8Array | undefined;
  type?: MoveStructTag | undefined;
  typeStr?: string | undefined;
  data?: string | undefined;
}

export interface WriteTableData {
  key?: string | undefined;
  keyType?: string | undefined;
  value?: string | undefined;
  valueType?: string | undefined;
}

export interface WriteTableItem {
  stateKeyHash?: Uint8Array | undefined;
  handle?: string | undefined;
  key?: string | undefined;
  data?: WriteTableData | undefined;
}

/**
 * Question: Not sure if this is the correct way to add extra config in protobuf here.
 * Not sure about the numbering as well. Please double check.
 */
export interface TransactionPayload {
  type?: TransactionPayload_Type | undefined;
  entryFunctionPayload?: EntryFunctionPayload | undefined;
  scriptPayload?: ScriptPayload | undefined;
  writeSetPayload?: WriteSetPayload | undefined;
  multisigPayload?: MultisigPayload | undefined;
  extraConfigV1?: ExtraConfigV1 | undefined;
}

export enum TransactionPayload_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ENTRY_FUNCTION_PAYLOAD = 1,
  TYPE_SCRIPT_PAYLOAD = 2,
  TYPE_WRITE_SET_PAYLOAD = 4,
  TYPE_MULTISIG_PAYLOAD = 5,
  UNRECOGNIZED = -1,
}

export function transactionPayload_TypeFromJSON(object: any): TransactionPayload_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return TransactionPayload_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ENTRY_FUNCTION_PAYLOAD":
      return TransactionPayload_Type.TYPE_ENTRY_FUNCTION_PAYLOAD;
    case 2:
    case "TYPE_SCRIPT_PAYLOAD":
      return TransactionPayload_Type.TYPE_SCRIPT_PAYLOAD;
    case 4:
    case "TYPE_WRITE_SET_PAYLOAD":
      return TransactionPayload_Type.TYPE_WRITE_SET_PAYLOAD;
    case 5:
    case "TYPE_MULTISIG_PAYLOAD":
      return TransactionPayload_Type.TYPE_MULTISIG_PAYLOAD;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TransactionPayload_Type.UNRECOGNIZED;
  }
}

export function transactionPayload_TypeToJSON(object: TransactionPayload_Type): string {
  switch (object) {
    case TransactionPayload_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case TransactionPayload_Type.TYPE_ENTRY_FUNCTION_PAYLOAD:
      return "TYPE_ENTRY_FUNCTION_PAYLOAD";
    case TransactionPayload_Type.TYPE_SCRIPT_PAYLOAD:
      return "TYPE_SCRIPT_PAYLOAD";
    case TransactionPayload_Type.TYPE_WRITE_SET_PAYLOAD:
      return "TYPE_WRITE_SET_PAYLOAD";
    case TransactionPayload_Type.TYPE_MULTISIG_PAYLOAD:
      return "TYPE_MULTISIG_PAYLOAD";
    case TransactionPayload_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface ExtraConfigV1 {
  multisigAddress?: string | undefined;
  replayProtectionNonce?: bigint | undefined;
}

export interface EntryFunctionPayload {
  function?: EntryFunctionId | undefined;
  typeArguments?: MoveType[] | undefined;
  arguments?: string[] | undefined;
  entryFunctionIdStr?: string | undefined;
}

export interface MoveScriptBytecode {
  bytecode?: Uint8Array | undefined;
  abi?: MoveFunction | undefined;
}

export interface ScriptPayload {
  code?: MoveScriptBytecode | undefined;
  typeArguments?: MoveType[] | undefined;
  arguments?: string[] | undefined;
}

export interface MultisigPayload {
  multisigAddress?: string | undefined;
  transactionPayload?: MultisigTransactionPayload | undefined;
}

export interface MultisigTransactionPayload {
  type?: MultisigTransactionPayload_Type | undefined;
  entryFunctionPayload?: EntryFunctionPayload | undefined;
}

export enum MultisigTransactionPayload_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ENTRY_FUNCTION_PAYLOAD = 1,
  UNRECOGNIZED = -1,
}

export function multisigTransactionPayload_TypeFromJSON(object: any): MultisigTransactionPayload_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return MultisigTransactionPayload_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ENTRY_FUNCTION_PAYLOAD":
      return MultisigTransactionPayload_Type.TYPE_ENTRY_FUNCTION_PAYLOAD;
    case -1:
    case "UNRECOGNIZED":
    default:
      return MultisigTransactionPayload_Type.UNRECOGNIZED;
  }
}

export function multisigTransactionPayload_TypeToJSON(object: MultisigTransactionPayload_Type): string {
  switch (object) {
    case MultisigTransactionPayload_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case MultisigTransactionPayload_Type.TYPE_ENTRY_FUNCTION_PAYLOAD:
      return "TYPE_ENTRY_FUNCTION_PAYLOAD";
    case MultisigTransactionPayload_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface MoveModuleBytecode {
  bytecode?: Uint8Array | undefined;
  abi?: MoveModule | undefined;
}

export interface MoveModule {
  address?: string | undefined;
  name?: string | undefined;
  friends?: MoveModuleId[] | undefined;
  exposedFunctions?: MoveFunction[] | undefined;
  structs?: MoveStruct[] | undefined;
}

export interface MoveFunction {
  name?: string | undefined;
  visibility?: MoveFunction_Visibility | undefined;
  isEntry?: boolean | undefined;
  genericTypeParams?: MoveFunctionGenericTypeParam[] | undefined;
  params?: MoveType[] | undefined;
  return?: MoveType[] | undefined;
}

export enum MoveFunction_Visibility {
  VISIBILITY_UNSPECIFIED = 0,
  VISIBILITY_PRIVATE = 1,
  VISIBILITY_PUBLIC = 2,
  VISIBILITY_FRIEND = 3,
  UNRECOGNIZED = -1,
}

export function moveFunction_VisibilityFromJSON(object: any): MoveFunction_Visibility {
  switch (object) {
    case 0:
    case "VISIBILITY_UNSPECIFIED":
      return MoveFunction_Visibility.VISIBILITY_UNSPECIFIED;
    case 1:
    case "VISIBILITY_PRIVATE":
      return MoveFunction_Visibility.VISIBILITY_PRIVATE;
    case 2:
    case "VISIBILITY_PUBLIC":
      return MoveFunction_Visibility.VISIBILITY_PUBLIC;
    case 3:
    case "VISIBILITY_FRIEND":
      return MoveFunction_Visibility.VISIBILITY_FRIEND;
    case -1:
    case "UNRECOGNIZED":
    default:
      return MoveFunction_Visibility.UNRECOGNIZED;
  }
}

export function moveFunction_VisibilityToJSON(object: MoveFunction_Visibility): string {
  switch (object) {
    case MoveFunction_Visibility.VISIBILITY_UNSPECIFIED:
      return "VISIBILITY_UNSPECIFIED";
    case MoveFunction_Visibility.VISIBILITY_PRIVATE:
      return "VISIBILITY_PRIVATE";
    case MoveFunction_Visibility.VISIBILITY_PUBLIC:
      return "VISIBILITY_PUBLIC";
    case MoveFunction_Visibility.VISIBILITY_FRIEND:
      return "VISIBILITY_FRIEND";
    case MoveFunction_Visibility.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface MoveStruct {
  name?: string | undefined;
  isNative?: boolean | undefined;
  isEvent?: boolean | undefined;
  abilities?: MoveAbility[] | undefined;
  genericTypeParams?: MoveStructGenericTypeParam[] | undefined;
  fields?: MoveStructField[] | undefined;
}

export interface MoveStructGenericTypeParam {
  constraints?: MoveAbility[] | undefined;
  isPhantom?: boolean | undefined;
}

export interface MoveStructField {
  name?: string | undefined;
  type?: MoveType | undefined;
}

export interface MoveFunctionGenericTypeParam {
  constraints?: MoveAbility[] | undefined;
}

export interface MoveType {
  type?: MoveTypes | undefined;
  vector?: MoveType | undefined;
  struct?: MoveStructTag | undefined;
  genericTypeParamIndex?: number | undefined;
  reference?: MoveType_ReferenceType | undefined;
  unparsable?: string | undefined;
}

export interface MoveType_ReferenceType {
  mutable?: boolean | undefined;
  to?: MoveType | undefined;
}

export interface WriteSetPayload {
  writeSet?: WriteSet | undefined;
}

export interface EntryFunctionId {
  module?: MoveModuleId | undefined;
  name?: string | undefined;
}

export interface MoveModuleId {
  address?: string | undefined;
  name?: string | undefined;
}

export interface MoveStructTag {
  address?: string | undefined;
  module?: string | undefined;
  name?: string | undefined;
  genericTypeParams?: MoveType[] | undefined;
}

export interface Signature {
  type?: Signature_Type | undefined;
  ed25519?: Ed25519Signature | undefined;
  multiEd25519?: MultiEd25519Signature | undefined;
  multiAgent?: MultiAgentSignature | undefined;
  feePayer?:
    | FeePayerSignature
    | undefined;
  /** 6 is reserved. */
  singleSender?: SingleSender | undefined;
}

export enum Signature_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ED25519 = 1,
  TYPE_MULTI_ED25519 = 2,
  TYPE_MULTI_AGENT = 3,
  TYPE_FEE_PAYER = 4,
  TYPE_SINGLE_SENDER = 6,
  UNRECOGNIZED = -1,
}

export function signature_TypeFromJSON(object: any): Signature_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return Signature_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ED25519":
      return Signature_Type.TYPE_ED25519;
    case 2:
    case "TYPE_MULTI_ED25519":
      return Signature_Type.TYPE_MULTI_ED25519;
    case 3:
    case "TYPE_MULTI_AGENT":
      return Signature_Type.TYPE_MULTI_AGENT;
    case 4:
    case "TYPE_FEE_PAYER":
      return Signature_Type.TYPE_FEE_PAYER;
    case 6:
    case "TYPE_SINGLE_SENDER":
      return Signature_Type.TYPE_SINGLE_SENDER;
    case -1:
    case "UNRECOGNIZED":
    default:
      return Signature_Type.UNRECOGNIZED;
  }
}

export function signature_TypeToJSON(object: Signature_Type): string {
  switch (object) {
    case Signature_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case Signature_Type.TYPE_ED25519:
      return "TYPE_ED25519";
    case Signature_Type.TYPE_MULTI_ED25519:
      return "TYPE_MULTI_ED25519";
    case Signature_Type.TYPE_MULTI_AGENT:
      return "TYPE_MULTI_AGENT";
    case Signature_Type.TYPE_FEE_PAYER:
      return "TYPE_FEE_PAYER";
    case Signature_Type.TYPE_SINGLE_SENDER:
      return "TYPE_SINGLE_SENDER";
    case Signature_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface Ed25519Signature {
  publicKey?: Uint8Array | undefined;
  signature?: Uint8Array | undefined;
}

export interface MultiEd25519Signature {
  publicKeys?: Uint8Array[] | undefined;
  signatures?: Uint8Array[] | undefined;
  threshold?: number | undefined;
  publicKeyIndices?: number[] | undefined;
}

export interface MultiAgentSignature {
  sender?: AccountSignature | undefined;
  secondarySignerAddresses?: string[] | undefined;
  secondarySigners?: AccountSignature[] | undefined;
}

export interface FeePayerSignature {
  sender?: AccountSignature | undefined;
  secondarySignerAddresses?: string[] | undefined;
  secondarySigners?: AccountSignature[] | undefined;
  feePayerAddress?: string | undefined;
  feePayerSigner?: AccountSignature | undefined;
}

export interface AnyPublicKey {
  type?: AnyPublicKey_Type | undefined;
  publicKey?: Uint8Array | undefined;
}

export enum AnyPublicKey_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ED25519 = 1,
  TYPE_SECP256K1_ECDSA = 2,
  TYPE_SECP256R1_ECDSA = 3,
  TYPE_KEYLESS = 4,
  TYPE_FEDERATED_KEYLESS = 5,
  UNRECOGNIZED = -1,
}

export function anyPublicKey_TypeFromJSON(object: any): AnyPublicKey_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return AnyPublicKey_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ED25519":
      return AnyPublicKey_Type.TYPE_ED25519;
    case 2:
    case "TYPE_SECP256K1_ECDSA":
      return AnyPublicKey_Type.TYPE_SECP256K1_ECDSA;
    case 3:
    case "TYPE_SECP256R1_ECDSA":
      return AnyPublicKey_Type.TYPE_SECP256R1_ECDSA;
    case 4:
    case "TYPE_KEYLESS":
      return AnyPublicKey_Type.TYPE_KEYLESS;
    case 5:
    case "TYPE_FEDERATED_KEYLESS":
      return AnyPublicKey_Type.TYPE_FEDERATED_KEYLESS;
    case -1:
    case "UNRECOGNIZED":
    default:
      return AnyPublicKey_Type.UNRECOGNIZED;
  }
}

export function anyPublicKey_TypeToJSON(object: AnyPublicKey_Type): string {
  switch (object) {
    case AnyPublicKey_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case AnyPublicKey_Type.TYPE_ED25519:
      return "TYPE_ED25519";
    case AnyPublicKey_Type.TYPE_SECP256K1_ECDSA:
      return "TYPE_SECP256K1_ECDSA";
    case AnyPublicKey_Type.TYPE_SECP256R1_ECDSA:
      return "TYPE_SECP256R1_ECDSA";
    case AnyPublicKey_Type.TYPE_KEYLESS:
      return "TYPE_KEYLESS";
    case AnyPublicKey_Type.TYPE_FEDERATED_KEYLESS:
      return "TYPE_FEDERATED_KEYLESS";
    case AnyPublicKey_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface AnySignature {
  type?:
    | AnySignature_Type
    | undefined;
  /**
   * Deprecated: use signature_variant instead.
   * Note: >= 1.10, this field is deprecated.
   *
   * @deprecated
   */
  signature?: Uint8Array | undefined;
  ed25519?: Ed25519 | undefined;
  secp256k1Ecdsa?: Secp256k1Ecdsa | undefined;
  webauthn?: WebAuthn | undefined;
  keyless?: Keyless | undefined;
}

export enum AnySignature_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ED25519 = 1,
  TYPE_SECP256K1_ECDSA = 2,
  TYPE_WEBAUTHN = 3,
  TYPE_KEYLESS = 4,
  UNRECOGNIZED = -1,
}

export function anySignature_TypeFromJSON(object: any): AnySignature_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return AnySignature_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ED25519":
      return AnySignature_Type.TYPE_ED25519;
    case 2:
    case "TYPE_SECP256K1_ECDSA":
      return AnySignature_Type.TYPE_SECP256K1_ECDSA;
    case 3:
    case "TYPE_WEBAUTHN":
      return AnySignature_Type.TYPE_WEBAUTHN;
    case 4:
    case "TYPE_KEYLESS":
      return AnySignature_Type.TYPE_KEYLESS;
    case -1:
    case "UNRECOGNIZED":
    default:
      return AnySignature_Type.UNRECOGNIZED;
  }
}

export function anySignature_TypeToJSON(object: AnySignature_Type): string {
  switch (object) {
    case AnySignature_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case AnySignature_Type.TYPE_ED25519:
      return "TYPE_ED25519";
    case AnySignature_Type.TYPE_SECP256K1_ECDSA:
      return "TYPE_SECP256K1_ECDSA";
    case AnySignature_Type.TYPE_WEBAUTHN:
      return "TYPE_WEBAUTHN";
    case AnySignature_Type.TYPE_KEYLESS:
      return "TYPE_KEYLESS";
    case AnySignature_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface Ed25519 {
  signature?: Uint8Array | undefined;
}

export interface Secp256k1Ecdsa {
  signature?: Uint8Array | undefined;
}

export interface WebAuthn {
  signature?: Uint8Array | undefined;
}

export interface Keyless {
  signature?: Uint8Array | undefined;
}

export interface SingleKeySignature {
  publicKey?: AnyPublicKey | undefined;
  signature?: AnySignature | undefined;
}

export interface IndexedSignature {
  index?: number | undefined;
  signature?: AnySignature | undefined;
}

export interface MultiKeySignature {
  publicKeys?: AnyPublicKey[] | undefined;
  signatures?: IndexedSignature[] | undefined;
  signaturesRequired?: number | undefined;
}

export interface AbstractSignature {
  functionInfo?: string | undefined;
  signature?: Uint8Array | undefined;
}

export interface SingleSender {
  sender?: AccountSignature | undefined;
}

export interface AccountSignature {
  type?: AccountSignature_Type | undefined;
  ed25519?: Ed25519Signature | undefined;
  multiEd25519?:
    | MultiEd25519Signature
    | undefined;
  /** 4 is reserved. */
  singleKeySignature?: SingleKeySignature | undefined;
  multiKeySignature?: MultiKeySignature | undefined;
  abstraction?: AbstractSignature | undefined;
}

export enum AccountSignature_Type {
  TYPE_UNSPECIFIED = 0,
  TYPE_ED25519 = 1,
  TYPE_MULTI_ED25519 = 2,
  TYPE_SINGLE_KEY = 4,
  TYPE_MULTI_KEY = 5,
  TYPE_ABSTRACTION = 6,
  UNRECOGNIZED = -1,
}

export function accountSignature_TypeFromJSON(object: any): AccountSignature_Type {
  switch (object) {
    case 0:
    case "TYPE_UNSPECIFIED":
      return AccountSignature_Type.TYPE_UNSPECIFIED;
    case 1:
    case "TYPE_ED25519":
      return AccountSignature_Type.TYPE_ED25519;
    case 2:
    case "TYPE_MULTI_ED25519":
      return AccountSignature_Type.TYPE_MULTI_ED25519;
    case 4:
    case "TYPE_SINGLE_KEY":
      return AccountSignature_Type.TYPE_SINGLE_KEY;
    case 5:
    case "TYPE_MULTI_KEY":
      return AccountSignature_Type.TYPE_MULTI_KEY;
    case 6:
    case "TYPE_ABSTRACTION":
      return AccountSignature_Type.TYPE_ABSTRACTION;
    case -1:
    case "UNRECOGNIZED":
    default:
      return AccountSignature_Type.UNRECOGNIZED;
  }
}

export function accountSignature_TypeToJSON(object: AccountSignature_Type): string {
  switch (object) {
    case AccountSignature_Type.TYPE_UNSPECIFIED:
      return "TYPE_UNSPECIFIED";
    case AccountSignature_Type.TYPE_ED25519:
      return "TYPE_ED25519";
    case AccountSignature_Type.TYPE_MULTI_ED25519:
      return "TYPE_MULTI_ED25519";
    case AccountSignature_Type.TYPE_SINGLE_KEY:
      return "TYPE_SINGLE_KEY";
    case AccountSignature_Type.TYPE_MULTI_KEY:
      return "TYPE_MULTI_KEY";
    case AccountSignature_Type.TYPE_ABSTRACTION:
      return "TYPE_ABSTRACTION";
    case AccountSignature_Type.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface TransactionSizeInfo {
  transactionBytes?: number | undefined;
  eventSizeInfo?: EventSizeInfo[] | undefined;
  writeOpSizeInfo?: WriteOpSizeInfo[] | undefined;
}

export interface EventSizeInfo {
  typeTagBytes?: number | undefined;
  totalBytes?: number | undefined;
}

export interface WriteOpSizeInfo {
  keyBytes?: number | undefined;
  valueBytes?: number | undefined;
}

function createBaseBlock(): Block {
  return { timestamp: undefined, height: BigInt("0"), transactions: [], chainId: 0 };
}

export const Block = {
  encode(message: Block, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(10).fork()).ldelim();
    }
    if (message.height !== undefined && message.height !== BigInt("0")) {
      if (BigInt.asUintN(64, message.height) !== message.height) {
        throw new globalThis.Error("value provided for field message.height of type uint64 too large");
      }
      writer.uint32(16).uint64(message.height.toString());
    }
    if (message.transactions !== undefined && message.transactions.length !== 0) {
      for (const v of message.transactions) {
        Transaction.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    if (message.chainId !== undefined && message.chainId !== 0) {
      writer.uint32(32).uint32(message.chainId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Block {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBlock();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.timestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.height = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.transactions!.push(Transaction.decode(reader, reader.uint32()));
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.chainId = reader.uint32();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Block, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Block | Block[]> | Iterable<Block | Block[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Block.encode(p).finish()];
        }
      } else {
        yield* [Block.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Block>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Block> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Block.decode(p)];
        }
      } else {
        yield* [Block.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Block {
    return {
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      height: isSet(object.height) ? BigInt(object.height) : BigInt("0"),
      transactions: globalThis.Array.isArray(object?.transactions)
        ? object.transactions.map((e: any) => Transaction.fromJSON(e))
        : [],
      chainId: isSet(object.chainId) ? globalThis.Number(object.chainId) : 0,
    };
  },

  toJSON(message: Block): unknown {
    const obj: any = {};
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.height !== undefined && message.height !== BigInt("0")) {
      obj.height = message.height.toString();
    }
    if (message.transactions?.length) {
      obj.transactions = message.transactions.map((e) => Transaction.toJSON(e));
    }
    if (message.chainId !== undefined && message.chainId !== 0) {
      obj.chainId = Math.round(message.chainId);
    }
    return obj;
  },

  create(base?: DeepPartial<Block>): Block {
    return Block.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Block>): Block {
    const message = createBaseBlock();
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.height = object.height ?? BigInt("0");
    message.transactions = object.transactions?.map((e) => Transaction.fromPartial(e)) || [];
    message.chainId = object.chainId ?? 0;
    return message;
  },
};

function createBaseTransaction(): Transaction {
  return {
    timestamp: undefined,
    version: BigInt("0"),
    info: undefined,
    epoch: BigInt("0"),
    blockHeight: BigInt("0"),
    type: 0,
    blockMetadata: undefined,
    genesis: undefined,
    stateCheckpoint: undefined,
    user: undefined,
    validator: undefined,
    blockEpilogue: undefined,
    sizeInfo: undefined,
  };
}

export const Transaction = {
  encode(message: Transaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(10).fork()).ldelim();
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      if (BigInt.asUintN(64, message.version) !== message.version) {
        throw new globalThis.Error("value provided for field message.version of type uint64 too large");
      }
      writer.uint32(16).uint64(message.version.toString());
    }
    if (message.info !== undefined) {
      TransactionInfo.encode(message.info, writer.uint32(26).fork()).ldelim();
    }
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      if (BigInt.asUintN(64, message.epoch) !== message.epoch) {
        throw new globalThis.Error("value provided for field message.epoch of type uint64 too large");
      }
      writer.uint32(32).uint64(message.epoch.toString());
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      if (BigInt.asUintN(64, message.blockHeight) !== message.blockHeight) {
        throw new globalThis.Error("value provided for field message.blockHeight of type uint64 too large");
      }
      writer.uint32(40).uint64(message.blockHeight.toString());
    }
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(48).int32(message.type);
    }
    if (message.blockMetadata !== undefined) {
      BlockMetadataTransaction.encode(message.blockMetadata, writer.uint32(58).fork()).ldelim();
    }
    if (message.genesis !== undefined) {
      GenesisTransaction.encode(message.genesis, writer.uint32(66).fork()).ldelim();
    }
    if (message.stateCheckpoint !== undefined) {
      StateCheckpointTransaction.encode(message.stateCheckpoint, writer.uint32(74).fork()).ldelim();
    }
    if (message.user !== undefined) {
      UserTransaction.encode(message.user, writer.uint32(82).fork()).ldelim();
    }
    if (message.validator !== undefined) {
      ValidatorTransaction.encode(message.validator, writer.uint32(170).fork()).ldelim();
    }
    if (message.blockEpilogue !== undefined) {
      BlockEpilogueTransaction.encode(message.blockEpilogue, writer.uint32(186).fork()).ldelim();
    }
    if (message.sizeInfo !== undefined) {
      TransactionSizeInfo.encode(message.sizeInfo, writer.uint32(178).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Transaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.timestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.version = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.info = TransactionInfo.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.epoch = longToBigint(reader.uint64() as Long);
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.blockHeight = longToBigint(reader.uint64() as Long);
          continue;
        case 6:
          if (tag !== 48) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.blockMetadata = BlockMetadataTransaction.decode(reader, reader.uint32());
          continue;
        case 8:
          if (tag !== 66) {
            break;
          }

          message.genesis = GenesisTransaction.decode(reader, reader.uint32());
          continue;
        case 9:
          if (tag !== 74) {
            break;
          }

          message.stateCheckpoint = StateCheckpointTransaction.decode(reader, reader.uint32());
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.user = UserTransaction.decode(reader, reader.uint32());
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.validator = ValidatorTransaction.decode(reader, reader.uint32());
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.blockEpilogue = BlockEpilogueTransaction.decode(reader, reader.uint32());
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.sizeInfo = TransactionSizeInfo.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Transaction, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Transaction | Transaction[]> | Iterable<Transaction | Transaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Transaction.encode(p).finish()];
        }
      } else {
        yield* [Transaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Transaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Transaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Transaction.decode(p)];
        }
      } else {
        yield* [Transaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Transaction {
    return {
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      version: isSet(object.version) ? BigInt(object.version) : BigInt("0"),
      info: isSet(object.info) ? TransactionInfo.fromJSON(object.info) : undefined,
      epoch: isSet(object.epoch) ? BigInt(object.epoch) : BigInt("0"),
      blockHeight: isSet(object.blockHeight) ? BigInt(object.blockHeight) : BigInt("0"),
      type: isSet(object.type) ? transaction_TransactionTypeFromJSON(object.type) : 0,
      blockMetadata: isSet(object.blockMetadata) ? BlockMetadataTransaction.fromJSON(object.blockMetadata) : undefined,
      genesis: isSet(object.genesis) ? GenesisTransaction.fromJSON(object.genesis) : undefined,
      stateCheckpoint: isSet(object.stateCheckpoint)
        ? StateCheckpointTransaction.fromJSON(object.stateCheckpoint)
        : undefined,
      user: isSet(object.user) ? UserTransaction.fromJSON(object.user) : undefined,
      validator: isSet(object.validator) ? ValidatorTransaction.fromJSON(object.validator) : undefined,
      blockEpilogue: isSet(object.blockEpilogue) ? BlockEpilogueTransaction.fromJSON(object.blockEpilogue) : undefined,
      sizeInfo: isSet(object.sizeInfo) ? TransactionSizeInfo.fromJSON(object.sizeInfo) : undefined,
    };
  },

  toJSON(message: Transaction): unknown {
    const obj: any = {};
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      obj.version = message.version.toString();
    }
    if (message.info !== undefined) {
      obj.info = TransactionInfo.toJSON(message.info);
    }
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      obj.epoch = message.epoch.toString();
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      obj.blockHeight = message.blockHeight.toString();
    }
    if (message.type !== undefined && message.type !== 0) {
      obj.type = transaction_TransactionTypeToJSON(message.type);
    }
    if (message.blockMetadata !== undefined) {
      obj.blockMetadata = BlockMetadataTransaction.toJSON(message.blockMetadata);
    }
    if (message.genesis !== undefined) {
      obj.genesis = GenesisTransaction.toJSON(message.genesis);
    }
    if (message.stateCheckpoint !== undefined) {
      obj.stateCheckpoint = StateCheckpointTransaction.toJSON(message.stateCheckpoint);
    }
    if (message.user !== undefined) {
      obj.user = UserTransaction.toJSON(message.user);
    }
    if (message.validator !== undefined) {
      obj.validator = ValidatorTransaction.toJSON(message.validator);
    }
    if (message.blockEpilogue !== undefined) {
      obj.blockEpilogue = BlockEpilogueTransaction.toJSON(message.blockEpilogue);
    }
    if (message.sizeInfo !== undefined) {
      obj.sizeInfo = TransactionSizeInfo.toJSON(message.sizeInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<Transaction>): Transaction {
    return Transaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Transaction>): Transaction {
    const message = createBaseTransaction();
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.version = object.version ?? BigInt("0");
    message.info = (object.info !== undefined && object.info !== null)
      ? TransactionInfo.fromPartial(object.info)
      : undefined;
    message.epoch = object.epoch ?? BigInt("0");
    message.blockHeight = object.blockHeight ?? BigInt("0");
    message.type = object.type ?? 0;
    message.blockMetadata = (object.blockMetadata !== undefined && object.blockMetadata !== null)
      ? BlockMetadataTransaction.fromPartial(object.blockMetadata)
      : undefined;
    message.genesis = (object.genesis !== undefined && object.genesis !== null)
      ? GenesisTransaction.fromPartial(object.genesis)
      : undefined;
    message.stateCheckpoint = (object.stateCheckpoint !== undefined && object.stateCheckpoint !== null)
      ? StateCheckpointTransaction.fromPartial(object.stateCheckpoint)
      : undefined;
    message.user = (object.user !== undefined && object.user !== null)
      ? UserTransaction.fromPartial(object.user)
      : undefined;
    message.validator = (object.validator !== undefined && object.validator !== null)
      ? ValidatorTransaction.fromPartial(object.validator)
      : undefined;
    message.blockEpilogue = (object.blockEpilogue !== undefined && object.blockEpilogue !== null)
      ? BlockEpilogueTransaction.fromPartial(object.blockEpilogue)
      : undefined;
    message.sizeInfo = (object.sizeInfo !== undefined && object.sizeInfo !== null)
      ? TransactionSizeInfo.fromPartial(object.sizeInfo)
      : undefined;
    return message;
  },
};

function createBaseBlockMetadataTransaction(): BlockMetadataTransaction {
  return {
    id: "",
    round: BigInt("0"),
    events: [],
    previousBlockVotesBitvec: new Uint8Array(0),
    proposer: "",
    failedProposerIndices: [],
  };
}

export const BlockMetadataTransaction = {
  encode(message: BlockMetadataTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== undefined && message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.round !== undefined && message.round !== BigInt("0")) {
      if (BigInt.asUintN(64, message.round) !== message.round) {
        throw new globalThis.Error("value provided for field message.round of type uint64 too large");
      }
      writer.uint32(16).uint64(message.round.toString());
    }
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    if (message.previousBlockVotesBitvec !== undefined && message.previousBlockVotesBitvec.length !== 0) {
      writer.uint32(34).bytes(message.previousBlockVotesBitvec);
    }
    if (message.proposer !== undefined && message.proposer !== "") {
      writer.uint32(42).string(message.proposer);
    }
    if (message.failedProposerIndices !== undefined && message.failedProposerIndices.length !== 0) {
      writer.uint32(50).fork();
      for (const v of message.failedProposerIndices) {
        writer.uint32(v);
      }
      writer.ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BlockMetadataTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBlockMetadataTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.round = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.previousBlockVotesBitvec = reader.bytes();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.proposer = reader.string();
          continue;
        case 6:
          if (tag === 48) {
            message.failedProposerIndices!.push(reader.uint32());

            continue;
          }

          if (tag === 50) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.failedProposerIndices!.push(reader.uint32());
            }

            continue;
          }

          break;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<BlockMetadataTransaction, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<BlockMetadataTransaction | BlockMetadataTransaction[]>
      | Iterable<BlockMetadataTransaction | BlockMetadataTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockMetadataTransaction.encode(p).finish()];
        }
      } else {
        yield* [BlockMetadataTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, BlockMetadataTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<BlockMetadataTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockMetadataTransaction.decode(p)];
        }
      } else {
        yield* [BlockMetadataTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): BlockMetadataTransaction {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      round: isSet(object.round) ? BigInt(object.round) : BigInt("0"),
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
      previousBlockVotesBitvec: isSet(object.previousBlockVotesBitvec)
        ? bytesFromBase64(object.previousBlockVotesBitvec)
        : new Uint8Array(0),
      proposer: isSet(object.proposer) ? globalThis.String(object.proposer) : "",
      failedProposerIndices: globalThis.Array.isArray(object?.failedProposerIndices)
        ? object.failedProposerIndices.map((e: any) => globalThis.Number(e))
        : [],
    };
  },

  toJSON(message: BlockMetadataTransaction): unknown {
    const obj: any = {};
    if (message.id !== undefined && message.id !== "") {
      obj.id = message.id;
    }
    if (message.round !== undefined && message.round !== BigInt("0")) {
      obj.round = message.round.toString();
    }
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    if (message.previousBlockVotesBitvec !== undefined && message.previousBlockVotesBitvec.length !== 0) {
      obj.previousBlockVotesBitvec = base64FromBytes(message.previousBlockVotesBitvec);
    }
    if (message.proposer !== undefined && message.proposer !== "") {
      obj.proposer = message.proposer;
    }
    if (message.failedProposerIndices?.length) {
      obj.failedProposerIndices = message.failedProposerIndices.map((e) => Math.round(e));
    }
    return obj;
  },

  create(base?: DeepPartial<BlockMetadataTransaction>): BlockMetadataTransaction {
    return BlockMetadataTransaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<BlockMetadataTransaction>): BlockMetadataTransaction {
    const message = createBaseBlockMetadataTransaction();
    message.id = object.id ?? "";
    message.round = object.round ?? BigInt("0");
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    message.previousBlockVotesBitvec = object.previousBlockVotesBitvec ?? new Uint8Array(0);
    message.proposer = object.proposer ?? "";
    message.failedProposerIndices = object.failedProposerIndices?.map((e) => e) || [];
    return message;
  },
};

function createBaseGenesisTransaction(): GenesisTransaction {
  return { payload: undefined, events: [] };
}

export const GenesisTransaction = {
  encode(message: GenesisTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.payload !== undefined) {
      WriteSet.encode(message.payload, writer.uint32(10).fork()).ldelim();
    }
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GenesisTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGenesisTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.payload = WriteSet.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<GenesisTransaction, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<GenesisTransaction | GenesisTransaction[]>
      | Iterable<GenesisTransaction | GenesisTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GenesisTransaction.encode(p).finish()];
        }
      } else {
        yield* [GenesisTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GenesisTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GenesisTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GenesisTransaction.decode(p)];
        }
      } else {
        yield* [GenesisTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GenesisTransaction {
    return {
      payload: isSet(object.payload) ? WriteSet.fromJSON(object.payload) : undefined,
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
    };
  },

  toJSON(message: GenesisTransaction): unknown {
    const obj: any = {};
    if (message.payload !== undefined) {
      obj.payload = WriteSet.toJSON(message.payload);
    }
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<GenesisTransaction>): GenesisTransaction {
    return GenesisTransaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GenesisTransaction>): GenesisTransaction {
    const message = createBaseGenesisTransaction();
    message.payload = (object.payload !== undefined && object.payload !== null)
      ? WriteSet.fromPartial(object.payload)
      : undefined;
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    return message;
  },
};

function createBaseStateCheckpointTransaction(): StateCheckpointTransaction {
  return {};
}

export const StateCheckpointTransaction = {
  encode(_: StateCheckpointTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StateCheckpointTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStateCheckpointTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<StateCheckpointTransaction, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<StateCheckpointTransaction | StateCheckpointTransaction[]>
      | Iterable<StateCheckpointTransaction | StateCheckpointTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StateCheckpointTransaction.encode(p).finish()];
        }
      } else {
        yield* [StateCheckpointTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, StateCheckpointTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<StateCheckpointTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StateCheckpointTransaction.decode(p)];
        }
      } else {
        yield* [StateCheckpointTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(_: any): StateCheckpointTransaction {
    return {};
  },

  toJSON(_: StateCheckpointTransaction): unknown {
    const obj: any = {};
    return obj;
  },

  create(base?: DeepPartial<StateCheckpointTransaction>): StateCheckpointTransaction {
    return StateCheckpointTransaction.fromPartial(base ?? {});
  },
  fromPartial(_: DeepPartial<StateCheckpointTransaction>): StateCheckpointTransaction {
    const message = createBaseStateCheckpointTransaction();
    return message;
  },
};

function createBaseValidatorTransaction(): ValidatorTransaction {
  return { observedJwkUpdate: undefined, dkgUpdate: undefined, events: [] };
}

export const ValidatorTransaction = {
  encode(message: ValidatorTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.observedJwkUpdate !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate.encode(message.observedJwkUpdate, writer.uint32(10).fork()).ldelim();
    }
    if (message.dkgUpdate !== undefined) {
      ValidatorTransaction_DkgUpdate.encode(message.dkgUpdate, writer.uint32(18).fork()).ldelim();
    }
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ValidatorTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.observedJwkUpdate = ValidatorTransaction_ObservedJwkUpdate.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.dkgUpdate = ValidatorTransaction_DkgUpdate.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<ValidatorTransaction | ValidatorTransaction[]>
      | Iterable<ValidatorTransaction | ValidatorTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction {
    return {
      observedJwkUpdate: isSet(object.observedJwkUpdate)
        ? ValidatorTransaction_ObservedJwkUpdate.fromJSON(object.observedJwkUpdate)
        : undefined,
      dkgUpdate: isSet(object.dkgUpdate) ? ValidatorTransaction_DkgUpdate.fromJSON(object.dkgUpdate) : undefined,
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
    };
  },

  toJSON(message: ValidatorTransaction): unknown {
    const obj: any = {};
    if (message.observedJwkUpdate !== undefined) {
      obj.observedJwkUpdate = ValidatorTransaction_ObservedJwkUpdate.toJSON(message.observedJwkUpdate);
    }
    if (message.dkgUpdate !== undefined) {
      obj.dkgUpdate = ValidatorTransaction_DkgUpdate.toJSON(message.dkgUpdate);
    }
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<ValidatorTransaction>): ValidatorTransaction {
    return ValidatorTransaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ValidatorTransaction>): ValidatorTransaction {
    const message = createBaseValidatorTransaction();
    message.observedJwkUpdate = (object.observedJwkUpdate !== undefined && object.observedJwkUpdate !== null)
      ? ValidatorTransaction_ObservedJwkUpdate.fromPartial(object.observedJwkUpdate)
      : undefined;
    message.dkgUpdate = (object.dkgUpdate !== undefined && object.dkgUpdate !== null)
      ? ValidatorTransaction_DkgUpdate.fromPartial(object.dkgUpdate)
      : undefined;
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate(): ValidatorTransaction_ObservedJwkUpdate {
  return { quorumCertifiedUpdate: undefined };
}

export const ValidatorTransaction_ObservedJwkUpdate = {
  encode(message: ValidatorTransaction_ObservedJwkUpdate, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.quorumCertifiedUpdate !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.encode(
        message.quorumCertifiedUpdate,
        writer.uint32(10).fork(),
      ).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ValidatorTransaction_ObservedJwkUpdate {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.quorumCertifiedUpdate = ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.decode(
            reader,
            reader.uint32(),
          );
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<ValidatorTransaction_ObservedJwkUpdate | ValidatorTransaction_ObservedJwkUpdate[]>
      | Iterable<ValidatorTransaction_ObservedJwkUpdate | ValidatorTransaction_ObservedJwkUpdate[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate {
    return {
      quorumCertifiedUpdate: isSet(object.quorumCertifiedUpdate)
        ? ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.fromJSON(object.quorumCertifiedUpdate)
        : undefined,
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate): unknown {
    const obj: any = {};
    if (message.quorumCertifiedUpdate !== undefined) {
      obj.quorumCertifiedUpdate = ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.toJSON(
        message.quorumCertifiedUpdate,
      );
    }
    return obj;
  },

  create(base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate>): ValidatorTransaction_ObservedJwkUpdate {
    return ValidatorTransaction_ObservedJwkUpdate.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate>): ValidatorTransaction_ObservedJwkUpdate {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate();
    message.quorumCertifiedUpdate =
      (object.quorumCertifiedUpdate !== undefined && object.quorumCertifiedUpdate !== null)
        ? ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.fromPartial(object.quorumCertifiedUpdate)
        : undefined;
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs(): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
  return { issuer: "", version: BigInt("0"), jwks: [] };
}

export const ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.issuer !== undefined && message.issuer !== "") {
      writer.uint32(10).string(message.issuer);
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      if (BigInt.asUintN(64, message.version) !== message.version) {
        throw new globalThis.Error("value provided for field message.version of type uint64 too large");
      }
      writer.uint32(16).uint64(message.version.toString());
    }
    if (message.jwks !== undefined && message.jwks.length !== 0) {
      for (const v of message.jwks) {
        ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.issuer = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.version = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.jwks!.push(
            ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.decode(reader, reader.uint32()),
          );
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
    return {
      issuer: isSet(object.issuer) ? globalThis.String(object.issuer) : "",
      version: isSet(object.version) ? BigInt(object.version) : BigInt("0"),
      jwks: globalThis.Array.isArray(object?.jwks)
        ? object.jwks.map((e: any) => ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.fromJSON(e))
        : [],
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs): unknown {
    const obj: any = {};
    if (message.issuer !== undefined && message.issuer !== "") {
      obj.issuer = message.issuer;
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      obj.version = message.version.toString();
    }
    if (message.jwks?.length) {
      obj.jwks = message.jwks.map((e) => ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.toJSON(e));
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
    return ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs();
    message.issuer = object.issuer ?? "";
    message.version = object.version ?? BigInt("0");
    message.jwks =
      object.jwks?.map((e) => ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.fromPartial(e)) || [];
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK(): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
  return { unsupportedJwk: undefined, rsa: undefined };
}

export const ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.unsupportedJwk !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.encode(
        message.unsupportedJwk,
        writer.uint32(10).fork(),
      ).ldelim();
    }
    if (message.rsa !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.encode(message.rsa, writer.uint32(18).fork())
        .ldelim();
    }
    return writer;
  },

  decode(
    input: _m0.Reader | Uint8Array,
    length?: number,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.unsupportedJwk = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK
            .decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.rsa = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.decode(
            reader,
            reader.uint32(),
          );
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
    return {
      unsupportedJwk: isSet(object.unsupportedJwk)
        ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.fromJSON(object.unsupportedJwk)
        : undefined,
      rsa: isSet(object.rsa)
        ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.fromJSON(object.rsa)
        : undefined,
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK): unknown {
    const obj: any = {};
    if (message.unsupportedJwk !== undefined) {
      obj.unsupportedJwk = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.toJSON(
        message.unsupportedJwk,
      );
    }
    if (message.rsa !== undefined) {
      obj.rsa = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.toJSON(message.rsa);
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
    return ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK();
    message.unsupportedJwk = (object.unsupportedJwk !== undefined && object.unsupportedJwk !== null)
      ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.fromPartial(
        object.unsupportedJwk,
      )
      : undefined;
    message.rsa = (object.rsa !== undefined && object.rsa !== null)
      ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.fromPartial(object.rsa)
      : undefined;
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA(): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
  return { kid: "", kty: "", alg: "", e: "", n: "" };
}

export const ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.kid !== undefined && message.kid !== "") {
      writer.uint32(10).string(message.kid);
    }
    if (message.kty !== undefined && message.kty !== "") {
      writer.uint32(18).string(message.kty);
    }
    if (message.alg !== undefined && message.alg !== "") {
      writer.uint32(26).string(message.alg);
    }
    if (message.e !== undefined && message.e !== "") {
      writer.uint32(34).string(message.e);
    }
    if (message.n !== undefined && message.n !== "") {
      writer.uint32(42).string(message.n);
    }
    return writer;
  },

  decode(
    input: _m0.Reader | Uint8Array,
    length?: number,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.kid = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.kty = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.alg = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.e = reader.string();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.n = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
    return {
      kid: isSet(object.kid) ? globalThis.String(object.kid) : "",
      kty: isSet(object.kty) ? globalThis.String(object.kty) : "",
      alg: isSet(object.alg) ? globalThis.String(object.alg) : "",
      e: isSet(object.e) ? globalThis.String(object.e) : "",
      n: isSet(object.n) ? globalThis.String(object.n) : "",
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA): unknown {
    const obj: any = {};
    if (message.kid !== undefined && message.kid !== "") {
      obj.kid = message.kid;
    }
    if (message.kty !== undefined && message.kty !== "") {
      obj.kty = message.kty;
    }
    if (message.alg !== undefined && message.alg !== "") {
      obj.alg = message.alg;
    }
    if (message.e !== undefined && message.e !== "") {
      obj.e = message.e;
    }
    if (message.n !== undefined && message.n !== "") {
      obj.n = message.n;
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
    return ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_RSA();
    message.kid = object.kid ?? "";
    message.kty = object.kty ?? "";
    message.alg = object.alg ?? "";
    message.e = object.e ?? "";
    message.n = object.n ?? "";
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK(): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
  return { id: new Uint8Array(0), payload: new Uint8Array(0) };
}

export const ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.id !== undefined && message.id.length !== 0) {
      writer.uint32(10).bytes(message.id);
    }
    if (message.payload !== undefined && message.payload.length !== 0) {
      writer.uint32(18).bytes(message.payload);
    }
    return writer;
  },

  decode(
    input: _m0.Reader | Uint8Array,
    length?: number,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.payload = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK
        | ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.encode(p).finish()];
        }
      } else {
        yield* [
          ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.encode(pkt as any).finish(),
        ];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
    return {
      id: isSet(object.id) ? bytesFromBase64(object.id) : new Uint8Array(0),
      payload: isSet(object.payload) ? bytesFromBase64(object.payload) : new Uint8Array(0),
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK): unknown {
    const obj: any = {};
    if (message.id !== undefined && message.id.length !== 0) {
      obj.id = base64FromBytes(message.id);
    }
    if (message.payload !== undefined && message.payload.length !== 0) {
      obj.payload = base64FromBytes(message.payload);
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
    return ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs_JWK_UnsupportedJWK();
    message.id = object.id ?? new Uint8Array(0);
    message.payload = object.payload ?? new Uint8Array(0);
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature(): ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
  return { signerIndices: [], sig: new Uint8Array(0) };
}

export const ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.signerIndices !== undefined && message.signerIndices.length !== 0) {
      writer.uint32(10).fork();
      for (const v of message.signerIndices) {
        if (BigInt.asUintN(64, v) !== v) {
          throw new globalThis.Error("a value provided in array field signerIndices of type uint64 is too large");
        }
        writer.uint64(v.toString());
      }
      writer.ldelim();
    }
    if (message.sig !== undefined && message.sig.length !== 0) {
      writer.uint32(18).bytes(message.sig);
    }
    return writer;
  },

  decode(
    input: _m0.Reader | Uint8Array,
    length?: number,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag === 8) {
            message.signerIndices!.push(longToBigint(reader.uint64() as Long));

            continue;
          }

          if (tag === 10) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.signerIndices!.push(longToBigint(reader.uint64() as Long));
            }

            continue;
          }

          break;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.sig = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature
        | ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature
        | ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
    return {
      signerIndices: globalThis.Array.isArray(object?.signerIndices)
        ? object.signerIndices.map((e: any) => BigInt(e))
        : [],
      sig: isSet(object.sig) ? bytesFromBase64(object.sig) : new Uint8Array(0),
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature): unknown {
    const obj: any = {};
    if (message.signerIndices?.length) {
      obj.signerIndices = message.signerIndices.map((e) => e.toString());
    }
    if (message.sig !== undefined && message.sig.length !== 0) {
      obj.sig = base64FromBytes(message.sig);
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
    return ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature>,
  ): ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature();
    message.signerIndices = object.signerIndices?.map((e) => e) || [];
    message.sig = object.sig ?? new Uint8Array(0);
    return message;
  },
};

function createBaseValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate(): ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
  return { update: undefined, multiSig: undefined };
}

export const ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate = {
  encode(
    message: ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate,
    writer: _m0.Writer = _m0.Writer.create(),
  ): _m0.Writer {
    if (message.update !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.encode(message.update, writer.uint32(10).fork())
        .ldelim();
    }
    if (message.multiSig !== undefined) {
      ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.encode(
        message.multiSig,
        writer.uint32(18).fork(),
      ).ldelim();
    }
    return writer;
  },

  decode(
    input: _m0.Reader | Uint8Array,
    length?: number,
  ): ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.update = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.multiSig = ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.decode(
            reader,
            reader.uint32(),
          );
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<
        | ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate
        | ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate[]
      >
      | Iterable<
        | ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate
        | ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate[]
      >,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
    return {
      update: isSet(object.update)
        ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.fromJSON(object.update)
        : undefined,
      multiSig: isSet(object.multiSig)
        ? ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.fromJSON(object.multiSig)
        : undefined,
    };
  },

  toJSON(message: ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate): unknown {
    const obj: any = {};
    if (message.update !== undefined) {
      obj.update = ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.toJSON(message.update);
    }
    if (message.multiSig !== undefined) {
      obj.multiSig = ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.toJSON(message.multiSig);
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate>,
  ): ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
    return ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate>,
  ): ValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate {
    const message = createBaseValidatorTransaction_ObservedJwkUpdate_QuorumCertifiedUpdate();
    message.update = (object.update !== undefined && object.update !== null)
      ? ValidatorTransaction_ObservedJwkUpdate_ExportedProviderJWKs.fromPartial(object.update)
      : undefined;
    message.multiSig = (object.multiSig !== undefined && object.multiSig !== null)
      ? ValidatorTransaction_ObservedJwkUpdate_ExportedAggregateSignature.fromPartial(object.multiSig)
      : undefined;
    return message;
  },
};

function createBaseValidatorTransaction_DkgUpdate(): ValidatorTransaction_DkgUpdate {
  return { dkgTranscript: undefined };
}

export const ValidatorTransaction_DkgUpdate = {
  encode(message: ValidatorTransaction_DkgUpdate, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.dkgTranscript !== undefined) {
      ValidatorTransaction_DkgUpdate_DkgTranscript.encode(message.dkgTranscript, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ValidatorTransaction_DkgUpdate {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_DkgUpdate();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.dkgTranscript = ValidatorTransaction_DkgUpdate_DkgTranscript.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_DkgUpdate, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<ValidatorTransaction_DkgUpdate | ValidatorTransaction_DkgUpdate[]>
      | Iterable<ValidatorTransaction_DkgUpdate | ValidatorTransaction_DkgUpdate[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_DkgUpdate.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_DkgUpdate.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_DkgUpdate>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_DkgUpdate> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_DkgUpdate.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_DkgUpdate.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_DkgUpdate {
    return {
      dkgTranscript: isSet(object.dkgTranscript)
        ? ValidatorTransaction_DkgUpdate_DkgTranscript.fromJSON(object.dkgTranscript)
        : undefined,
    };
  },

  toJSON(message: ValidatorTransaction_DkgUpdate): unknown {
    const obj: any = {};
    if (message.dkgTranscript !== undefined) {
      obj.dkgTranscript = ValidatorTransaction_DkgUpdate_DkgTranscript.toJSON(message.dkgTranscript);
    }
    return obj;
  },

  create(base?: DeepPartial<ValidatorTransaction_DkgUpdate>): ValidatorTransaction_DkgUpdate {
    return ValidatorTransaction_DkgUpdate.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ValidatorTransaction_DkgUpdate>): ValidatorTransaction_DkgUpdate {
    const message = createBaseValidatorTransaction_DkgUpdate();
    message.dkgTranscript = (object.dkgTranscript !== undefined && object.dkgTranscript !== null)
      ? ValidatorTransaction_DkgUpdate_DkgTranscript.fromPartial(object.dkgTranscript)
      : undefined;
    return message;
  },
};

function createBaseValidatorTransaction_DkgUpdate_DkgTranscript(): ValidatorTransaction_DkgUpdate_DkgTranscript {
  return { epoch: BigInt("0"), author: "", payload: new Uint8Array(0) };
}

export const ValidatorTransaction_DkgUpdate_DkgTranscript = {
  encode(message: ValidatorTransaction_DkgUpdate_DkgTranscript, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      if (BigInt.asUintN(64, message.epoch) !== message.epoch) {
        throw new globalThis.Error("value provided for field message.epoch of type uint64 too large");
      }
      writer.uint32(8).uint64(message.epoch.toString());
    }
    if (message.author !== undefined && message.author !== "") {
      writer.uint32(18).string(message.author);
    }
    if (message.payload !== undefined && message.payload.length !== 0) {
      writer.uint32(26).bytes(message.payload);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ValidatorTransaction_DkgUpdate_DkgTranscript {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseValidatorTransaction_DkgUpdate_DkgTranscript();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.epoch = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.author = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.payload = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ValidatorTransaction_DkgUpdate_DkgTranscript, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<ValidatorTransaction_DkgUpdate_DkgTranscript | ValidatorTransaction_DkgUpdate_DkgTranscript[]>
      | Iterable<ValidatorTransaction_DkgUpdate_DkgTranscript | ValidatorTransaction_DkgUpdate_DkgTranscript[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_DkgUpdate_DkgTranscript.encode(p).finish()];
        }
      } else {
        yield* [ValidatorTransaction_DkgUpdate_DkgTranscript.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ValidatorTransaction_DkgUpdate_DkgTranscript>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ValidatorTransaction_DkgUpdate_DkgTranscript> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ValidatorTransaction_DkgUpdate_DkgTranscript.decode(p)];
        }
      } else {
        yield* [ValidatorTransaction_DkgUpdate_DkgTranscript.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ValidatorTransaction_DkgUpdate_DkgTranscript {
    return {
      epoch: isSet(object.epoch) ? BigInt(object.epoch) : BigInt("0"),
      author: isSet(object.author) ? globalThis.String(object.author) : "",
      payload: isSet(object.payload) ? bytesFromBase64(object.payload) : new Uint8Array(0),
    };
  },

  toJSON(message: ValidatorTransaction_DkgUpdate_DkgTranscript): unknown {
    const obj: any = {};
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      obj.epoch = message.epoch.toString();
    }
    if (message.author !== undefined && message.author !== "") {
      obj.author = message.author;
    }
    if (message.payload !== undefined && message.payload.length !== 0) {
      obj.payload = base64FromBytes(message.payload);
    }
    return obj;
  },

  create(
    base?: DeepPartial<ValidatorTransaction_DkgUpdate_DkgTranscript>,
  ): ValidatorTransaction_DkgUpdate_DkgTranscript {
    return ValidatorTransaction_DkgUpdate_DkgTranscript.fromPartial(base ?? {});
  },
  fromPartial(
    object: DeepPartial<ValidatorTransaction_DkgUpdate_DkgTranscript>,
  ): ValidatorTransaction_DkgUpdate_DkgTranscript {
    const message = createBaseValidatorTransaction_DkgUpdate_DkgTranscript();
    message.epoch = object.epoch ?? BigInt("0");
    message.author = object.author ?? "";
    message.payload = object.payload ?? new Uint8Array(0);
    return message;
  },
};

function createBaseBlockEpilogueTransaction(): BlockEpilogueTransaction {
  return { blockEndInfo: undefined };
}

export const BlockEpilogueTransaction = {
  encode(message: BlockEpilogueTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.blockEndInfo !== undefined) {
      BlockEndInfo.encode(message.blockEndInfo, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BlockEpilogueTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBlockEpilogueTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.blockEndInfo = BlockEndInfo.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<BlockEpilogueTransaction, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<BlockEpilogueTransaction | BlockEpilogueTransaction[]>
      | Iterable<BlockEpilogueTransaction | BlockEpilogueTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockEpilogueTransaction.encode(p).finish()];
        }
      } else {
        yield* [BlockEpilogueTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, BlockEpilogueTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<BlockEpilogueTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockEpilogueTransaction.decode(p)];
        }
      } else {
        yield* [BlockEpilogueTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): BlockEpilogueTransaction {
    return { blockEndInfo: isSet(object.blockEndInfo) ? BlockEndInfo.fromJSON(object.blockEndInfo) : undefined };
  },

  toJSON(message: BlockEpilogueTransaction): unknown {
    const obj: any = {};
    if (message.blockEndInfo !== undefined) {
      obj.blockEndInfo = BlockEndInfo.toJSON(message.blockEndInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<BlockEpilogueTransaction>): BlockEpilogueTransaction {
    return BlockEpilogueTransaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<BlockEpilogueTransaction>): BlockEpilogueTransaction {
    const message = createBaseBlockEpilogueTransaction();
    message.blockEndInfo = (object.blockEndInfo !== undefined && object.blockEndInfo !== null)
      ? BlockEndInfo.fromPartial(object.blockEndInfo)
      : undefined;
    return message;
  },
};

function createBaseBlockEndInfo(): BlockEndInfo {
  return {
    blockGasLimitReached: false,
    blockOutputLimitReached: false,
    blockEffectiveBlockGasUnits: BigInt("0"),
    blockApproxOutputSize: BigInt("0"),
  };
}

export const BlockEndInfo = {
  encode(message: BlockEndInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.blockGasLimitReached === true) {
      writer.uint32(8).bool(message.blockGasLimitReached);
    }
    if (message.blockOutputLimitReached === true) {
      writer.uint32(16).bool(message.blockOutputLimitReached);
    }
    if (message.blockEffectiveBlockGasUnits !== undefined && message.blockEffectiveBlockGasUnits !== BigInt("0")) {
      if (BigInt.asUintN(64, message.blockEffectiveBlockGasUnits) !== message.blockEffectiveBlockGasUnits) {
        throw new globalThis.Error(
          "value provided for field message.blockEffectiveBlockGasUnits of type uint64 too large",
        );
      }
      writer.uint32(24).uint64(message.blockEffectiveBlockGasUnits.toString());
    }
    if (message.blockApproxOutputSize !== undefined && message.blockApproxOutputSize !== BigInt("0")) {
      if (BigInt.asUintN(64, message.blockApproxOutputSize) !== message.blockApproxOutputSize) {
        throw new globalThis.Error("value provided for field message.blockApproxOutputSize of type uint64 too large");
      }
      writer.uint32(32).uint64(message.blockApproxOutputSize.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BlockEndInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBlockEndInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.blockGasLimitReached = reader.bool();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.blockOutputLimitReached = reader.bool();
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.blockEffectiveBlockGasUnits = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.blockApproxOutputSize = longToBigint(reader.uint64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<BlockEndInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<BlockEndInfo | BlockEndInfo[]> | Iterable<BlockEndInfo | BlockEndInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockEndInfo.encode(p).finish()];
        }
      } else {
        yield* [BlockEndInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, BlockEndInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<BlockEndInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BlockEndInfo.decode(p)];
        }
      } else {
        yield* [BlockEndInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): BlockEndInfo {
    return {
      blockGasLimitReached: isSet(object.blockGasLimitReached)
        ? globalThis.Boolean(object.blockGasLimitReached)
        : false,
      blockOutputLimitReached: isSet(object.blockOutputLimitReached)
        ? globalThis.Boolean(object.blockOutputLimitReached)
        : false,
      blockEffectiveBlockGasUnits: isSet(object.blockEffectiveBlockGasUnits)
        ? BigInt(object.blockEffectiveBlockGasUnits)
        : BigInt("0"),
      blockApproxOutputSize: isSet(object.blockApproxOutputSize) ? BigInt(object.blockApproxOutputSize) : BigInt("0"),
    };
  },

  toJSON(message: BlockEndInfo): unknown {
    const obj: any = {};
    if (message.blockGasLimitReached === true) {
      obj.blockGasLimitReached = message.blockGasLimitReached;
    }
    if (message.blockOutputLimitReached === true) {
      obj.blockOutputLimitReached = message.blockOutputLimitReached;
    }
    if (message.blockEffectiveBlockGasUnits !== undefined && message.blockEffectiveBlockGasUnits !== BigInt("0")) {
      obj.blockEffectiveBlockGasUnits = message.blockEffectiveBlockGasUnits.toString();
    }
    if (message.blockApproxOutputSize !== undefined && message.blockApproxOutputSize !== BigInt("0")) {
      obj.blockApproxOutputSize = message.blockApproxOutputSize.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<BlockEndInfo>): BlockEndInfo {
    return BlockEndInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<BlockEndInfo>): BlockEndInfo {
    const message = createBaseBlockEndInfo();
    message.blockGasLimitReached = object.blockGasLimitReached ?? false;
    message.blockOutputLimitReached = object.blockOutputLimitReached ?? false;
    message.blockEffectiveBlockGasUnits = object.blockEffectiveBlockGasUnits ?? BigInt("0");
    message.blockApproxOutputSize = object.blockApproxOutputSize ?? BigInt("0");
    return message;
  },
};

function createBaseUserTransaction(): UserTransaction {
  return { request: undefined, events: [] };
}

export const UserTransaction = {
  encode(message: UserTransaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.request !== undefined) {
      UserTransactionRequest.encode(message.request, writer.uint32(10).fork()).ldelim();
    }
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UserTransaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUserTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.request = UserTransactionRequest.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<UserTransaction, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<UserTransaction | UserTransaction[]> | Iterable<UserTransaction | UserTransaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransaction.encode(p).finish()];
        }
      } else {
        yield* [UserTransaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, UserTransaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<UserTransaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransaction.decode(p)];
        }
      } else {
        yield* [UserTransaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): UserTransaction {
    return {
      request: isSet(object.request) ? UserTransactionRequest.fromJSON(object.request) : undefined,
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
    };
  },

  toJSON(message: UserTransaction): unknown {
    const obj: any = {};
    if (message.request !== undefined) {
      obj.request = UserTransactionRequest.toJSON(message.request);
    }
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<UserTransaction>): UserTransaction {
    return UserTransaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<UserTransaction>): UserTransaction {
    const message = createBaseUserTransaction();
    message.request = (object.request !== undefined && object.request !== null)
      ? UserTransactionRequest.fromPartial(object.request)
      : undefined;
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    return message;
  },
};

function createBaseEvent(): Event {
  return { key: undefined, sequenceNumber: BigInt("0"), type: undefined, typeStr: "", data: "" };
}

export const Event = {
  encode(message: Event, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== undefined) {
      EventKey.encode(message.key, writer.uint32(10).fork()).ldelim();
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      if (BigInt.asUintN(64, message.sequenceNumber) !== message.sequenceNumber) {
        throw new globalThis.Error("value provided for field message.sequenceNumber of type uint64 too large");
      }
      writer.uint32(16).uint64(message.sequenceNumber.toString());
    }
    if (message.type !== undefined) {
      MoveType.encode(message.type, writer.uint32(26).fork()).ldelim();
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      writer.uint32(42).string(message.typeStr);
    }
    if (message.data !== undefined && message.data !== "") {
      writer.uint32(34).string(message.data);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Event {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEvent();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = EventKey.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.sequenceNumber = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.type = MoveType.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.typeStr = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.data = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Event, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Event | Event[]> | Iterable<Event | Event[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Event.encode(p).finish()];
        }
      } else {
        yield* [Event.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Event>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Event> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Event.decode(p)];
        }
      } else {
        yield* [Event.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Event {
    return {
      key: isSet(object.key) ? EventKey.fromJSON(object.key) : undefined,
      sequenceNumber: isSet(object.sequenceNumber) ? BigInt(object.sequenceNumber) : BigInt("0"),
      type: isSet(object.type) ? MoveType.fromJSON(object.type) : undefined,
      typeStr: isSet(object.typeStr) ? globalThis.String(object.typeStr) : "",
      data: isSet(object.data) ? globalThis.String(object.data) : "",
    };
  },

  toJSON(message: Event): unknown {
    const obj: any = {};
    if (message.key !== undefined) {
      obj.key = EventKey.toJSON(message.key);
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      obj.sequenceNumber = message.sequenceNumber.toString();
    }
    if (message.type !== undefined) {
      obj.type = MoveType.toJSON(message.type);
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      obj.typeStr = message.typeStr;
    }
    if (message.data !== undefined && message.data !== "") {
      obj.data = message.data;
    }
    return obj;
  },

  create(base?: DeepPartial<Event>): Event {
    return Event.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Event>): Event {
    const message = createBaseEvent();
    message.key = (object.key !== undefined && object.key !== null) ? EventKey.fromPartial(object.key) : undefined;
    message.sequenceNumber = object.sequenceNumber ?? BigInt("0");
    message.type = (object.type !== undefined && object.type !== null) ? MoveType.fromPartial(object.type) : undefined;
    message.typeStr = object.typeStr ?? "";
    message.data = object.data ?? "";
    return message;
  },
};

function createBaseTransactionInfo(): TransactionInfo {
  return {
    hash: new Uint8Array(0),
    stateChangeHash: new Uint8Array(0),
    eventRootHash: new Uint8Array(0),
    stateCheckpointHash: undefined,
    gasUsed: BigInt("0"),
    success: false,
    vmStatus: "",
    accumulatorRootHash: new Uint8Array(0),
    changes: [],
  };
}

export const TransactionInfo = {
  encode(message: TransactionInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.hash !== undefined && message.hash.length !== 0) {
      writer.uint32(10).bytes(message.hash);
    }
    if (message.stateChangeHash !== undefined && message.stateChangeHash.length !== 0) {
      writer.uint32(18).bytes(message.stateChangeHash);
    }
    if (message.eventRootHash !== undefined && message.eventRootHash.length !== 0) {
      writer.uint32(26).bytes(message.eventRootHash);
    }
    if (message.stateCheckpointHash !== undefined) {
      writer.uint32(34).bytes(message.stateCheckpointHash);
    }
    if (message.gasUsed !== undefined && message.gasUsed !== BigInt("0")) {
      if (BigInt.asUintN(64, message.gasUsed) !== message.gasUsed) {
        throw new globalThis.Error("value provided for field message.gasUsed of type uint64 too large");
      }
      writer.uint32(40).uint64(message.gasUsed.toString());
    }
    if (message.success === true) {
      writer.uint32(48).bool(message.success);
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      writer.uint32(58).string(message.vmStatus);
    }
    if (message.accumulatorRootHash !== undefined && message.accumulatorRootHash.length !== 0) {
      writer.uint32(66).bytes(message.accumulatorRootHash);
    }
    if (message.changes !== undefined && message.changes.length !== 0) {
      for (const v of message.changes) {
        WriteSetChange.encode(v!, writer.uint32(74).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.hash = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.stateChangeHash = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.eventRootHash = reader.bytes();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.stateCheckpointHash = reader.bytes();
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.gasUsed = longToBigint(reader.uint64() as Long);
          continue;
        case 6:
          if (tag !== 48) {
            break;
          }

          message.success = reader.bool();
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.vmStatus = reader.string();
          continue;
        case 8:
          if (tag !== 66) {
            break;
          }

          message.accumulatorRootHash = reader.bytes();
          continue;
        case 9:
          if (tag !== 74) {
            break;
          }

          message.changes!.push(WriteSetChange.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<TransactionInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<TransactionInfo | TransactionInfo[]> | Iterable<TransactionInfo | TransactionInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionInfo.encode(p).finish()];
        }
      } else {
        yield* [TransactionInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionInfo.decode(p)];
        }
      } else {
        yield* [TransactionInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionInfo {
    return {
      hash: isSet(object.hash) ? bytesFromBase64(object.hash) : new Uint8Array(0),
      stateChangeHash: isSet(object.stateChangeHash) ? bytesFromBase64(object.stateChangeHash) : new Uint8Array(0),
      eventRootHash: isSet(object.eventRootHash) ? bytesFromBase64(object.eventRootHash) : new Uint8Array(0),
      stateCheckpointHash: isSet(object.stateCheckpointHash) ? bytesFromBase64(object.stateCheckpointHash) : undefined,
      gasUsed: isSet(object.gasUsed) ? BigInt(object.gasUsed) : BigInt("0"),
      success: isSet(object.success) ? globalThis.Boolean(object.success) : false,
      vmStatus: isSet(object.vmStatus) ? globalThis.String(object.vmStatus) : "",
      accumulatorRootHash: isSet(object.accumulatorRootHash)
        ? bytesFromBase64(object.accumulatorRootHash)
        : new Uint8Array(0),
      changes: globalThis.Array.isArray(object?.changes)
        ? object.changes.map((e: any) => WriteSetChange.fromJSON(e))
        : [],
    };
  },

  toJSON(message: TransactionInfo): unknown {
    const obj: any = {};
    if (message.hash !== undefined && message.hash.length !== 0) {
      obj.hash = base64FromBytes(message.hash);
    }
    if (message.stateChangeHash !== undefined && message.stateChangeHash.length !== 0) {
      obj.stateChangeHash = base64FromBytes(message.stateChangeHash);
    }
    if (message.eventRootHash !== undefined && message.eventRootHash.length !== 0) {
      obj.eventRootHash = base64FromBytes(message.eventRootHash);
    }
    if (message.stateCheckpointHash !== undefined) {
      obj.stateCheckpointHash = base64FromBytes(message.stateCheckpointHash);
    }
    if (message.gasUsed !== undefined && message.gasUsed !== BigInt("0")) {
      obj.gasUsed = message.gasUsed.toString();
    }
    if (message.success === true) {
      obj.success = message.success;
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      obj.vmStatus = message.vmStatus;
    }
    if (message.accumulatorRootHash !== undefined && message.accumulatorRootHash.length !== 0) {
      obj.accumulatorRootHash = base64FromBytes(message.accumulatorRootHash);
    }
    if (message.changes?.length) {
      obj.changes = message.changes.map((e) => WriteSetChange.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionInfo>): TransactionInfo {
    return TransactionInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionInfo>): TransactionInfo {
    const message = createBaseTransactionInfo();
    message.hash = object.hash ?? new Uint8Array(0);
    message.stateChangeHash = object.stateChangeHash ?? new Uint8Array(0);
    message.eventRootHash = object.eventRootHash ?? new Uint8Array(0);
    message.stateCheckpointHash = object.stateCheckpointHash ?? undefined;
    message.gasUsed = object.gasUsed ?? BigInt("0");
    message.success = object.success ?? false;
    message.vmStatus = object.vmStatus ?? "";
    message.accumulatorRootHash = object.accumulatorRootHash ?? new Uint8Array(0);
    message.changes = object.changes?.map((e) => WriteSetChange.fromPartial(e)) || [];
    return message;
  },
};

function createBaseEventKey(): EventKey {
  return { creationNumber: BigInt("0"), accountAddress: "" };
}

export const EventKey = {
  encode(message: EventKey, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.creationNumber !== undefined && message.creationNumber !== BigInt("0")) {
      if (BigInt.asUintN(64, message.creationNumber) !== message.creationNumber) {
        throw new globalThis.Error("value provided for field message.creationNumber of type uint64 too large");
      }
      writer.uint32(8).uint64(message.creationNumber.toString());
    }
    if (message.accountAddress !== undefined && message.accountAddress !== "") {
      writer.uint32(18).string(message.accountAddress);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EventKey {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventKey();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.creationNumber = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.accountAddress = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<EventKey, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventKey | EventKey[]> | Iterable<EventKey | EventKey[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventKey.encode(p).finish()];
        }
      } else {
        yield* [EventKey.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventKey>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventKey> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventKey.decode(p)];
        }
      } else {
        yield* [EventKey.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventKey {
    return {
      creationNumber: isSet(object.creationNumber) ? BigInt(object.creationNumber) : BigInt("0"),
      accountAddress: isSet(object.accountAddress) ? globalThis.String(object.accountAddress) : "",
    };
  },

  toJSON(message: EventKey): unknown {
    const obj: any = {};
    if (message.creationNumber !== undefined && message.creationNumber !== BigInt("0")) {
      obj.creationNumber = message.creationNumber.toString();
    }
    if (message.accountAddress !== undefined && message.accountAddress !== "") {
      obj.accountAddress = message.accountAddress;
    }
    return obj;
  },

  create(base?: DeepPartial<EventKey>): EventKey {
    return EventKey.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventKey>): EventKey {
    const message = createBaseEventKey();
    message.creationNumber = object.creationNumber ?? BigInt("0");
    message.accountAddress = object.accountAddress ?? "";
    return message;
  },
};

function createBaseUserTransactionRequest(): UserTransactionRequest {
  return {
    sender: "",
    sequenceNumber: BigInt("0"),
    maxGasAmount: BigInt("0"),
    gasUnitPrice: BigInt("0"),
    expirationTimestampSecs: undefined,
    payload: undefined,
    signature: undefined,
  };
}

export const UserTransactionRequest = {
  encode(message: UserTransactionRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sender !== undefined && message.sender !== "") {
      writer.uint32(10).string(message.sender);
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      if (BigInt.asUintN(64, message.sequenceNumber) !== message.sequenceNumber) {
        throw new globalThis.Error("value provided for field message.sequenceNumber of type uint64 too large");
      }
      writer.uint32(16).uint64(message.sequenceNumber.toString());
    }
    if (message.maxGasAmount !== undefined && message.maxGasAmount !== BigInt("0")) {
      if (BigInt.asUintN(64, message.maxGasAmount) !== message.maxGasAmount) {
        throw new globalThis.Error("value provided for field message.maxGasAmount of type uint64 too large");
      }
      writer.uint32(24).uint64(message.maxGasAmount.toString());
    }
    if (message.gasUnitPrice !== undefined && message.gasUnitPrice !== BigInt("0")) {
      if (BigInt.asUintN(64, message.gasUnitPrice) !== message.gasUnitPrice) {
        throw new globalThis.Error("value provided for field message.gasUnitPrice of type uint64 too large");
      }
      writer.uint32(32).uint64(message.gasUnitPrice.toString());
    }
    if (message.expirationTimestampSecs !== undefined) {
      Timestamp.encode(message.expirationTimestampSecs, writer.uint32(42).fork()).ldelim();
    }
    if (message.payload !== undefined) {
      TransactionPayload.encode(message.payload, writer.uint32(50).fork()).ldelim();
    }
    if (message.signature !== undefined) {
      Signature.encode(message.signature, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UserTransactionRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUserTransactionRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sender = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.sequenceNumber = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.maxGasAmount = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.gasUnitPrice = longToBigint(reader.uint64() as Long);
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.expirationTimestampSecs = Timestamp.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.payload = TransactionPayload.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.signature = Signature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<UserTransactionRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<UserTransactionRequest | UserTransactionRequest[]>
      | Iterable<UserTransactionRequest | UserTransactionRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionRequest.encode(p).finish()];
        }
      } else {
        yield* [UserTransactionRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, UserTransactionRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<UserTransactionRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionRequest.decode(p)];
        }
      } else {
        yield* [UserTransactionRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): UserTransactionRequest {
    return {
      sender: isSet(object.sender) ? globalThis.String(object.sender) : "",
      sequenceNumber: isSet(object.sequenceNumber) ? BigInt(object.sequenceNumber) : BigInt("0"),
      maxGasAmount: isSet(object.maxGasAmount) ? BigInt(object.maxGasAmount) : BigInt("0"),
      gasUnitPrice: isSet(object.gasUnitPrice) ? BigInt(object.gasUnitPrice) : BigInt("0"),
      expirationTimestampSecs: isSet(object.expirationTimestampSecs)
        ? Timestamp.fromJSON(object.expirationTimestampSecs)
        : undefined,
      payload: isSet(object.payload) ? TransactionPayload.fromJSON(object.payload) : undefined,
      signature: isSet(object.signature) ? Signature.fromJSON(object.signature) : undefined,
    };
  },

  toJSON(message: UserTransactionRequest): unknown {
    const obj: any = {};
    if (message.sender !== undefined && message.sender !== "") {
      obj.sender = message.sender;
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      obj.sequenceNumber = message.sequenceNumber.toString();
    }
    if (message.maxGasAmount !== undefined && message.maxGasAmount !== BigInt("0")) {
      obj.maxGasAmount = message.maxGasAmount.toString();
    }
    if (message.gasUnitPrice !== undefined && message.gasUnitPrice !== BigInt("0")) {
      obj.gasUnitPrice = message.gasUnitPrice.toString();
    }
    if (message.expirationTimestampSecs !== undefined) {
      obj.expirationTimestampSecs = Timestamp.toJSON(message.expirationTimestampSecs);
    }
    if (message.payload !== undefined) {
      obj.payload = TransactionPayload.toJSON(message.payload);
    }
    if (message.signature !== undefined) {
      obj.signature = Signature.toJSON(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<UserTransactionRequest>): UserTransactionRequest {
    return UserTransactionRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<UserTransactionRequest>): UserTransactionRequest {
    const message = createBaseUserTransactionRequest();
    message.sender = object.sender ?? "";
    message.sequenceNumber = object.sequenceNumber ?? BigInt("0");
    message.maxGasAmount = object.maxGasAmount ?? BigInt("0");
    message.gasUnitPrice = object.gasUnitPrice ?? BigInt("0");
    message.expirationTimestampSecs =
      (object.expirationTimestampSecs !== undefined && object.expirationTimestampSecs !== null)
        ? Timestamp.fromPartial(object.expirationTimestampSecs)
        : undefined;
    message.payload = (object.payload !== undefined && object.payload !== null)
      ? TransactionPayload.fromPartial(object.payload)
      : undefined;
    message.signature = (object.signature !== undefined && object.signature !== null)
      ? Signature.fromPartial(object.signature)
      : undefined;
    return message;
  },
};

function createBaseWriteSet(): WriteSet {
  return { writeSetType: 0, scriptWriteSet: undefined, directWriteSet: undefined };
}

export const WriteSet = {
  encode(message: WriteSet, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.writeSetType !== undefined && message.writeSetType !== 0) {
      writer.uint32(8).int32(message.writeSetType);
    }
    if (message.scriptWriteSet !== undefined) {
      ScriptWriteSet.encode(message.scriptWriteSet, writer.uint32(18).fork()).ldelim();
    }
    if (message.directWriteSet !== undefined) {
      DirectWriteSet.encode(message.directWriteSet, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteSet {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteSet();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.writeSetType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.scriptWriteSet = ScriptWriteSet.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.directWriteSet = DirectWriteSet.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteSet, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteSet | WriteSet[]> | Iterable<WriteSet | WriteSet[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSet.encode(p).finish()];
        }
      } else {
        yield* [WriteSet.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteSet>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteSet> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSet.decode(p)];
        }
      } else {
        yield* [WriteSet.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteSet {
    return {
      writeSetType: isSet(object.writeSetType) ? writeSet_WriteSetTypeFromJSON(object.writeSetType) : 0,
      scriptWriteSet: isSet(object.scriptWriteSet) ? ScriptWriteSet.fromJSON(object.scriptWriteSet) : undefined,
      directWriteSet: isSet(object.directWriteSet) ? DirectWriteSet.fromJSON(object.directWriteSet) : undefined,
    };
  },

  toJSON(message: WriteSet): unknown {
    const obj: any = {};
    if (message.writeSetType !== undefined && message.writeSetType !== 0) {
      obj.writeSetType = writeSet_WriteSetTypeToJSON(message.writeSetType);
    }
    if (message.scriptWriteSet !== undefined) {
      obj.scriptWriteSet = ScriptWriteSet.toJSON(message.scriptWriteSet);
    }
    if (message.directWriteSet !== undefined) {
      obj.directWriteSet = DirectWriteSet.toJSON(message.directWriteSet);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteSet>): WriteSet {
    return WriteSet.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteSet>): WriteSet {
    const message = createBaseWriteSet();
    message.writeSetType = object.writeSetType ?? 0;
    message.scriptWriteSet = (object.scriptWriteSet !== undefined && object.scriptWriteSet !== null)
      ? ScriptWriteSet.fromPartial(object.scriptWriteSet)
      : undefined;
    message.directWriteSet = (object.directWriteSet !== undefined && object.directWriteSet !== null)
      ? DirectWriteSet.fromPartial(object.directWriteSet)
      : undefined;
    return message;
  },
};

function createBaseScriptWriteSet(): ScriptWriteSet {
  return { executeAs: "", script: undefined };
}

export const ScriptWriteSet = {
  encode(message: ScriptWriteSet, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.executeAs !== undefined && message.executeAs !== "") {
      writer.uint32(10).string(message.executeAs);
    }
    if (message.script !== undefined) {
      ScriptPayload.encode(message.script, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ScriptWriteSet {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseScriptWriteSet();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.executeAs = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.script = ScriptPayload.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ScriptWriteSet, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ScriptWriteSet | ScriptWriteSet[]> | Iterable<ScriptWriteSet | ScriptWriteSet[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ScriptWriteSet.encode(p).finish()];
        }
      } else {
        yield* [ScriptWriteSet.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ScriptWriteSet>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ScriptWriteSet> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ScriptWriteSet.decode(p)];
        }
      } else {
        yield* [ScriptWriteSet.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ScriptWriteSet {
    return {
      executeAs: isSet(object.executeAs) ? globalThis.String(object.executeAs) : "",
      script: isSet(object.script) ? ScriptPayload.fromJSON(object.script) : undefined,
    };
  },

  toJSON(message: ScriptWriteSet): unknown {
    const obj: any = {};
    if (message.executeAs !== undefined && message.executeAs !== "") {
      obj.executeAs = message.executeAs;
    }
    if (message.script !== undefined) {
      obj.script = ScriptPayload.toJSON(message.script);
    }
    return obj;
  },

  create(base?: DeepPartial<ScriptWriteSet>): ScriptWriteSet {
    return ScriptWriteSet.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ScriptWriteSet>): ScriptWriteSet {
    const message = createBaseScriptWriteSet();
    message.executeAs = object.executeAs ?? "";
    message.script = (object.script !== undefined && object.script !== null)
      ? ScriptPayload.fromPartial(object.script)
      : undefined;
    return message;
  },
};

function createBaseDirectWriteSet(): DirectWriteSet {
  return { writeSetChange: [], events: [] };
}

export const DirectWriteSet = {
  encode(message: DirectWriteSet, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.writeSetChange !== undefined && message.writeSetChange.length !== 0) {
      for (const v of message.writeSetChange) {
        WriteSetChange.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DirectWriteSet {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDirectWriteSet();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.writeSetChange!.push(WriteSetChange.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<DirectWriteSet, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<DirectWriteSet | DirectWriteSet[]> | Iterable<DirectWriteSet | DirectWriteSet[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DirectWriteSet.encode(p).finish()];
        }
      } else {
        yield* [DirectWriteSet.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, DirectWriteSet>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<DirectWriteSet> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DirectWriteSet.decode(p)];
        }
      } else {
        yield* [DirectWriteSet.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): DirectWriteSet {
    return {
      writeSetChange: globalThis.Array.isArray(object?.writeSetChange)
        ? object.writeSetChange.map((e: any) => WriteSetChange.fromJSON(e))
        : [],
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
    };
  },

  toJSON(message: DirectWriteSet): unknown {
    const obj: any = {};
    if (message.writeSetChange?.length) {
      obj.writeSetChange = message.writeSetChange.map((e) => WriteSetChange.toJSON(e));
    }
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<DirectWriteSet>): DirectWriteSet {
    return DirectWriteSet.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<DirectWriteSet>): DirectWriteSet {
    const message = createBaseDirectWriteSet();
    message.writeSetChange = object.writeSetChange?.map((e) => WriteSetChange.fromPartial(e)) || [];
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    return message;
  },
};

function createBaseWriteSetChange(): WriteSetChange {
  return {
    type: 0,
    deleteModule: undefined,
    deleteResource: undefined,
    deleteTableItem: undefined,
    writeModule: undefined,
    writeResource: undefined,
    writeTableItem: undefined,
  };
}

export const WriteSetChange = {
  encode(message: WriteSetChange, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.deleteModule !== undefined) {
      DeleteModule.encode(message.deleteModule, writer.uint32(18).fork()).ldelim();
    }
    if (message.deleteResource !== undefined) {
      DeleteResource.encode(message.deleteResource, writer.uint32(26).fork()).ldelim();
    }
    if (message.deleteTableItem !== undefined) {
      DeleteTableItem.encode(message.deleteTableItem, writer.uint32(34).fork()).ldelim();
    }
    if (message.writeModule !== undefined) {
      WriteModule.encode(message.writeModule, writer.uint32(42).fork()).ldelim();
    }
    if (message.writeResource !== undefined) {
      WriteResource.encode(message.writeResource, writer.uint32(50).fork()).ldelim();
    }
    if (message.writeTableItem !== undefined) {
      WriteTableItem.encode(message.writeTableItem, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteSetChange {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteSetChange();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.deleteModule = DeleteModule.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.deleteResource = DeleteResource.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.deleteTableItem = DeleteTableItem.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.writeModule = WriteModule.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.writeResource = WriteResource.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.writeTableItem = WriteTableItem.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteSetChange, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteSetChange | WriteSetChange[]> | Iterable<WriteSetChange | WriteSetChange[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSetChange.encode(p).finish()];
        }
      } else {
        yield* [WriteSetChange.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteSetChange>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteSetChange> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSetChange.decode(p)];
        }
      } else {
        yield* [WriteSetChange.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteSetChange {
    return {
      type: isSet(object.type) ? writeSetChange_TypeFromJSON(object.type) : 0,
      deleteModule: isSet(object.deleteModule) ? DeleteModule.fromJSON(object.deleteModule) : undefined,
      deleteResource: isSet(object.deleteResource) ? DeleteResource.fromJSON(object.deleteResource) : undefined,
      deleteTableItem: isSet(object.deleteTableItem) ? DeleteTableItem.fromJSON(object.deleteTableItem) : undefined,
      writeModule: isSet(object.writeModule) ? WriteModule.fromJSON(object.writeModule) : undefined,
      writeResource: isSet(object.writeResource) ? WriteResource.fromJSON(object.writeResource) : undefined,
      writeTableItem: isSet(object.writeTableItem) ? WriteTableItem.fromJSON(object.writeTableItem) : undefined,
    };
  },

  toJSON(message: WriteSetChange): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = writeSetChange_TypeToJSON(message.type);
    }
    if (message.deleteModule !== undefined) {
      obj.deleteModule = DeleteModule.toJSON(message.deleteModule);
    }
    if (message.deleteResource !== undefined) {
      obj.deleteResource = DeleteResource.toJSON(message.deleteResource);
    }
    if (message.deleteTableItem !== undefined) {
      obj.deleteTableItem = DeleteTableItem.toJSON(message.deleteTableItem);
    }
    if (message.writeModule !== undefined) {
      obj.writeModule = WriteModule.toJSON(message.writeModule);
    }
    if (message.writeResource !== undefined) {
      obj.writeResource = WriteResource.toJSON(message.writeResource);
    }
    if (message.writeTableItem !== undefined) {
      obj.writeTableItem = WriteTableItem.toJSON(message.writeTableItem);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteSetChange>): WriteSetChange {
    return WriteSetChange.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteSetChange>): WriteSetChange {
    const message = createBaseWriteSetChange();
    message.type = object.type ?? 0;
    message.deleteModule = (object.deleteModule !== undefined && object.deleteModule !== null)
      ? DeleteModule.fromPartial(object.deleteModule)
      : undefined;
    message.deleteResource = (object.deleteResource !== undefined && object.deleteResource !== null)
      ? DeleteResource.fromPartial(object.deleteResource)
      : undefined;
    message.deleteTableItem = (object.deleteTableItem !== undefined && object.deleteTableItem !== null)
      ? DeleteTableItem.fromPartial(object.deleteTableItem)
      : undefined;
    message.writeModule = (object.writeModule !== undefined && object.writeModule !== null)
      ? WriteModule.fromPartial(object.writeModule)
      : undefined;
    message.writeResource = (object.writeResource !== undefined && object.writeResource !== null)
      ? WriteResource.fromPartial(object.writeResource)
      : undefined;
    message.writeTableItem = (object.writeTableItem !== undefined && object.writeTableItem !== null)
      ? WriteTableItem.fromPartial(object.writeTableItem)
      : undefined;
    return message;
  },
};

function createBaseDeleteModule(): DeleteModule {
  return { address: "", stateKeyHash: new Uint8Array(0), module: undefined };
}

export const DeleteModule = {
  encode(message: DeleteModule, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(18).bytes(message.stateKeyHash);
    }
    if (message.module !== undefined) {
      MoveModuleId.encode(message.module, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DeleteModule {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDeleteModule();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.module = MoveModuleId.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<DeleteModule, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<DeleteModule | DeleteModule[]> | Iterable<DeleteModule | DeleteModule[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteModule.encode(p).finish()];
        }
      } else {
        yield* [DeleteModule.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, DeleteModule>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<DeleteModule> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteModule.decode(p)];
        }
      } else {
        yield* [DeleteModule.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): DeleteModule {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      module: isSet(object.module) ? MoveModuleId.fromJSON(object.module) : undefined,
    };
  },

  toJSON(message: DeleteModule): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.module !== undefined) {
      obj.module = MoveModuleId.toJSON(message.module);
    }
    return obj;
  },

  create(base?: DeepPartial<DeleteModule>): DeleteModule {
    return DeleteModule.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<DeleteModule>): DeleteModule {
    const message = createBaseDeleteModule();
    message.address = object.address ?? "";
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.module = (object.module !== undefined && object.module !== null)
      ? MoveModuleId.fromPartial(object.module)
      : undefined;
    return message;
  },
};

function createBaseDeleteResource(): DeleteResource {
  return { address: "", stateKeyHash: new Uint8Array(0), type: undefined, typeStr: "" };
}

export const DeleteResource = {
  encode(message: DeleteResource, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(18).bytes(message.stateKeyHash);
    }
    if (message.type !== undefined) {
      MoveStructTag.encode(message.type, writer.uint32(26).fork()).ldelim();
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      writer.uint32(34).string(message.typeStr);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DeleteResource {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDeleteResource();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.type = MoveStructTag.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.typeStr = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<DeleteResource, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<DeleteResource | DeleteResource[]> | Iterable<DeleteResource | DeleteResource[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteResource.encode(p).finish()];
        }
      } else {
        yield* [DeleteResource.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, DeleteResource>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<DeleteResource> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteResource.decode(p)];
        }
      } else {
        yield* [DeleteResource.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): DeleteResource {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      type: isSet(object.type) ? MoveStructTag.fromJSON(object.type) : undefined,
      typeStr: isSet(object.typeStr) ? globalThis.String(object.typeStr) : "",
    };
  },

  toJSON(message: DeleteResource): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.type !== undefined) {
      obj.type = MoveStructTag.toJSON(message.type);
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      obj.typeStr = message.typeStr;
    }
    return obj;
  },

  create(base?: DeepPartial<DeleteResource>): DeleteResource {
    return DeleteResource.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<DeleteResource>): DeleteResource {
    const message = createBaseDeleteResource();
    message.address = object.address ?? "";
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.type = (object.type !== undefined && object.type !== null)
      ? MoveStructTag.fromPartial(object.type)
      : undefined;
    message.typeStr = object.typeStr ?? "";
    return message;
  },
};

function createBaseDeleteTableItem(): DeleteTableItem {
  return { stateKeyHash: new Uint8Array(0), handle: "", key: "", data: undefined };
}

export const DeleteTableItem = {
  encode(message: DeleteTableItem, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(10).bytes(message.stateKeyHash);
    }
    if (message.handle !== undefined && message.handle !== "") {
      writer.uint32(18).string(message.handle);
    }
    if (message.key !== undefined && message.key !== "") {
      writer.uint32(26).string(message.key);
    }
    if (message.data !== undefined) {
      DeleteTableData.encode(message.data, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DeleteTableItem {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDeleteTableItem();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.handle = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.key = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.data = DeleteTableData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<DeleteTableItem, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<DeleteTableItem | DeleteTableItem[]> | Iterable<DeleteTableItem | DeleteTableItem[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteTableItem.encode(p).finish()];
        }
      } else {
        yield* [DeleteTableItem.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, DeleteTableItem>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<DeleteTableItem> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteTableItem.decode(p)];
        }
      } else {
        yield* [DeleteTableItem.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): DeleteTableItem {
    return {
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      handle: isSet(object.handle) ? globalThis.String(object.handle) : "",
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      data: isSet(object.data) ? DeleteTableData.fromJSON(object.data) : undefined,
    };
  },

  toJSON(message: DeleteTableItem): unknown {
    const obj: any = {};
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.handle !== undefined && message.handle !== "") {
      obj.handle = message.handle;
    }
    if (message.key !== undefined && message.key !== "") {
      obj.key = message.key;
    }
    if (message.data !== undefined) {
      obj.data = DeleteTableData.toJSON(message.data);
    }
    return obj;
  },

  create(base?: DeepPartial<DeleteTableItem>): DeleteTableItem {
    return DeleteTableItem.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<DeleteTableItem>): DeleteTableItem {
    const message = createBaseDeleteTableItem();
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.handle = object.handle ?? "";
    message.key = object.key ?? "";
    message.data = (object.data !== undefined && object.data !== null)
      ? DeleteTableData.fromPartial(object.data)
      : undefined;
    return message;
  },
};

function createBaseDeleteTableData(): DeleteTableData {
  return { key: "", keyType: "" };
}

export const DeleteTableData = {
  encode(message: DeleteTableData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== undefined && message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.keyType !== undefined && message.keyType !== "") {
      writer.uint32(18).string(message.keyType);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DeleteTableData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDeleteTableData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.keyType = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<DeleteTableData, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<DeleteTableData | DeleteTableData[]> | Iterable<DeleteTableData | DeleteTableData[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteTableData.encode(p).finish()];
        }
      } else {
        yield* [DeleteTableData.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, DeleteTableData>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<DeleteTableData> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [DeleteTableData.decode(p)];
        }
      } else {
        yield* [DeleteTableData.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): DeleteTableData {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      keyType: isSet(object.keyType) ? globalThis.String(object.keyType) : "",
    };
  },

  toJSON(message: DeleteTableData): unknown {
    const obj: any = {};
    if (message.key !== undefined && message.key !== "") {
      obj.key = message.key;
    }
    if (message.keyType !== undefined && message.keyType !== "") {
      obj.keyType = message.keyType;
    }
    return obj;
  },

  create(base?: DeepPartial<DeleteTableData>): DeleteTableData {
    return DeleteTableData.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<DeleteTableData>): DeleteTableData {
    const message = createBaseDeleteTableData();
    message.key = object.key ?? "";
    message.keyType = object.keyType ?? "";
    return message;
  },
};

function createBaseWriteModule(): WriteModule {
  return { address: "", stateKeyHash: new Uint8Array(0), data: undefined };
}

export const WriteModule = {
  encode(message: WriteModule, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(18).bytes(message.stateKeyHash);
    }
    if (message.data !== undefined) {
      MoveModuleBytecode.encode(message.data, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteModule {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteModule();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.data = MoveModuleBytecode.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteModule, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteModule | WriteModule[]> | Iterable<WriteModule | WriteModule[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteModule.encode(p).finish()];
        }
      } else {
        yield* [WriteModule.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteModule>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteModule> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteModule.decode(p)];
        }
      } else {
        yield* [WriteModule.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteModule {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      data: isSet(object.data) ? MoveModuleBytecode.fromJSON(object.data) : undefined,
    };
  },

  toJSON(message: WriteModule): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.data !== undefined) {
      obj.data = MoveModuleBytecode.toJSON(message.data);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteModule>): WriteModule {
    return WriteModule.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteModule>): WriteModule {
    const message = createBaseWriteModule();
    message.address = object.address ?? "";
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.data = (object.data !== undefined && object.data !== null)
      ? MoveModuleBytecode.fromPartial(object.data)
      : undefined;
    return message;
  },
};

function createBaseWriteResource(): WriteResource {
  return { address: "", stateKeyHash: new Uint8Array(0), type: undefined, typeStr: "", data: "" };
}

export const WriteResource = {
  encode(message: WriteResource, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(18).bytes(message.stateKeyHash);
    }
    if (message.type !== undefined) {
      MoveStructTag.encode(message.type, writer.uint32(26).fork()).ldelim();
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      writer.uint32(34).string(message.typeStr);
    }
    if (message.data !== undefined && message.data !== "") {
      writer.uint32(42).string(message.data);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteResource {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteResource();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.type = MoveStructTag.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.typeStr = reader.string();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.data = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteResource, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteResource | WriteResource[]> | Iterable<WriteResource | WriteResource[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteResource.encode(p).finish()];
        }
      } else {
        yield* [WriteResource.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteResource>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteResource> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteResource.decode(p)];
        }
      } else {
        yield* [WriteResource.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteResource {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      type: isSet(object.type) ? MoveStructTag.fromJSON(object.type) : undefined,
      typeStr: isSet(object.typeStr) ? globalThis.String(object.typeStr) : "",
      data: isSet(object.data) ? globalThis.String(object.data) : "",
    };
  },

  toJSON(message: WriteResource): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.type !== undefined) {
      obj.type = MoveStructTag.toJSON(message.type);
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      obj.typeStr = message.typeStr;
    }
    if (message.data !== undefined && message.data !== "") {
      obj.data = message.data;
    }
    return obj;
  },

  create(base?: DeepPartial<WriteResource>): WriteResource {
    return WriteResource.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteResource>): WriteResource {
    const message = createBaseWriteResource();
    message.address = object.address ?? "";
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.type = (object.type !== undefined && object.type !== null)
      ? MoveStructTag.fromPartial(object.type)
      : undefined;
    message.typeStr = object.typeStr ?? "";
    message.data = object.data ?? "";
    return message;
  },
};

function createBaseWriteTableData(): WriteTableData {
  return { key: "", keyType: "", value: "", valueType: "" };
}

export const WriteTableData = {
  encode(message: WriteTableData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== undefined && message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.keyType !== undefined && message.keyType !== "") {
      writer.uint32(18).string(message.keyType);
    }
    if (message.value !== undefined && message.value !== "") {
      writer.uint32(26).string(message.value);
    }
    if (message.valueType !== undefined && message.valueType !== "") {
      writer.uint32(34).string(message.valueType);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteTableData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteTableData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.keyType = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.value = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.valueType = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteTableData, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteTableData | WriteTableData[]> | Iterable<WriteTableData | WriteTableData[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteTableData.encode(p).finish()];
        }
      } else {
        yield* [WriteTableData.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteTableData>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteTableData> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteTableData.decode(p)];
        }
      } else {
        yield* [WriteTableData.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteTableData {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      keyType: isSet(object.keyType) ? globalThis.String(object.keyType) : "",
      value: isSet(object.value) ? globalThis.String(object.value) : "",
      valueType: isSet(object.valueType) ? globalThis.String(object.valueType) : "",
    };
  },

  toJSON(message: WriteTableData): unknown {
    const obj: any = {};
    if (message.key !== undefined && message.key !== "") {
      obj.key = message.key;
    }
    if (message.keyType !== undefined && message.keyType !== "") {
      obj.keyType = message.keyType;
    }
    if (message.value !== undefined && message.value !== "") {
      obj.value = message.value;
    }
    if (message.valueType !== undefined && message.valueType !== "") {
      obj.valueType = message.valueType;
    }
    return obj;
  },

  create(base?: DeepPartial<WriteTableData>): WriteTableData {
    return WriteTableData.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteTableData>): WriteTableData {
    const message = createBaseWriteTableData();
    message.key = object.key ?? "";
    message.keyType = object.keyType ?? "";
    message.value = object.value ?? "";
    message.valueType = object.valueType ?? "";
    return message;
  },
};

function createBaseWriteTableItem(): WriteTableItem {
  return { stateKeyHash: new Uint8Array(0), handle: "", key: "", data: undefined };
}

export const WriteTableItem = {
  encode(message: WriteTableItem, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      writer.uint32(10).bytes(message.stateKeyHash);
    }
    if (message.handle !== undefined && message.handle !== "") {
      writer.uint32(18).string(message.handle);
    }
    if (message.key !== undefined && message.key !== "") {
      writer.uint32(26).string(message.key);
    }
    if (message.data !== undefined) {
      WriteTableData.encode(message.data, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteTableItem {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteTableItem();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.stateKeyHash = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.handle = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.key = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.data = WriteTableData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteTableItem, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteTableItem | WriteTableItem[]> | Iterable<WriteTableItem | WriteTableItem[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteTableItem.encode(p).finish()];
        }
      } else {
        yield* [WriteTableItem.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteTableItem>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteTableItem> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteTableItem.decode(p)];
        }
      } else {
        yield* [WriteTableItem.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteTableItem {
    return {
      stateKeyHash: isSet(object.stateKeyHash) ? bytesFromBase64(object.stateKeyHash) : new Uint8Array(0),
      handle: isSet(object.handle) ? globalThis.String(object.handle) : "",
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      data: isSet(object.data) ? WriteTableData.fromJSON(object.data) : undefined,
    };
  },

  toJSON(message: WriteTableItem): unknown {
    const obj: any = {};
    if (message.stateKeyHash !== undefined && message.stateKeyHash.length !== 0) {
      obj.stateKeyHash = base64FromBytes(message.stateKeyHash);
    }
    if (message.handle !== undefined && message.handle !== "") {
      obj.handle = message.handle;
    }
    if (message.key !== undefined && message.key !== "") {
      obj.key = message.key;
    }
    if (message.data !== undefined) {
      obj.data = WriteTableData.toJSON(message.data);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteTableItem>): WriteTableItem {
    return WriteTableItem.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteTableItem>): WriteTableItem {
    const message = createBaseWriteTableItem();
    message.stateKeyHash = object.stateKeyHash ?? new Uint8Array(0);
    message.handle = object.handle ?? "";
    message.key = object.key ?? "";
    message.data = (object.data !== undefined && object.data !== null)
      ? WriteTableData.fromPartial(object.data)
      : undefined;
    return message;
  },
};

function createBaseTransactionPayload(): TransactionPayload {
  return {
    type: 0,
    entryFunctionPayload: undefined,
    scriptPayload: undefined,
    writeSetPayload: undefined,
    multisigPayload: undefined,
    extraConfigV1: undefined,
  };
}

export const TransactionPayload = {
  encode(message: TransactionPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.entryFunctionPayload !== undefined) {
      EntryFunctionPayload.encode(message.entryFunctionPayload, writer.uint32(18).fork()).ldelim();
    }
    if (message.scriptPayload !== undefined) {
      ScriptPayload.encode(message.scriptPayload, writer.uint32(26).fork()).ldelim();
    }
    if (message.writeSetPayload !== undefined) {
      WriteSetPayload.encode(message.writeSetPayload, writer.uint32(42).fork()).ldelim();
    }
    if (message.multisigPayload !== undefined) {
      MultisigPayload.encode(message.multisigPayload, writer.uint32(50).fork()).ldelim();
    }
    if (message.extraConfigV1 !== undefined) {
      ExtraConfigV1.encode(message.extraConfigV1, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.entryFunctionPayload = EntryFunctionPayload.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.scriptPayload = ScriptPayload.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.writeSetPayload = WriteSetPayload.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.multisigPayload = MultisigPayload.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.extraConfigV1 = ExtraConfigV1.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<TransactionPayload, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionPayload | TransactionPayload[]>
      | Iterable<TransactionPayload | TransactionPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionPayload.encode(p).finish()];
        }
      } else {
        yield* [TransactionPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionPayload.decode(p)];
        }
      } else {
        yield* [TransactionPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionPayload {
    return {
      type: isSet(object.type) ? transactionPayload_TypeFromJSON(object.type) : 0,
      entryFunctionPayload: isSet(object.entryFunctionPayload)
        ? EntryFunctionPayload.fromJSON(object.entryFunctionPayload)
        : undefined,
      scriptPayload: isSet(object.scriptPayload) ? ScriptPayload.fromJSON(object.scriptPayload) : undefined,
      writeSetPayload: isSet(object.writeSetPayload) ? WriteSetPayload.fromJSON(object.writeSetPayload) : undefined,
      multisigPayload: isSet(object.multisigPayload) ? MultisigPayload.fromJSON(object.multisigPayload) : undefined,
      extraConfigV1: isSet(object.extraConfigV1) ? ExtraConfigV1.fromJSON(object.extraConfigV1) : undefined,
    };
  },

  toJSON(message: TransactionPayload): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = transactionPayload_TypeToJSON(message.type);
    }
    if (message.entryFunctionPayload !== undefined) {
      obj.entryFunctionPayload = EntryFunctionPayload.toJSON(message.entryFunctionPayload);
    }
    if (message.scriptPayload !== undefined) {
      obj.scriptPayload = ScriptPayload.toJSON(message.scriptPayload);
    }
    if (message.writeSetPayload !== undefined) {
      obj.writeSetPayload = WriteSetPayload.toJSON(message.writeSetPayload);
    }
    if (message.multisigPayload !== undefined) {
      obj.multisigPayload = MultisigPayload.toJSON(message.multisigPayload);
    }
    if (message.extraConfigV1 !== undefined) {
      obj.extraConfigV1 = ExtraConfigV1.toJSON(message.extraConfigV1);
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionPayload>): TransactionPayload {
    return TransactionPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionPayload>): TransactionPayload {
    const message = createBaseTransactionPayload();
    message.type = object.type ?? 0;
    message.entryFunctionPayload = (object.entryFunctionPayload !== undefined && object.entryFunctionPayload !== null)
      ? EntryFunctionPayload.fromPartial(object.entryFunctionPayload)
      : undefined;
    message.scriptPayload = (object.scriptPayload !== undefined && object.scriptPayload !== null)
      ? ScriptPayload.fromPartial(object.scriptPayload)
      : undefined;
    message.writeSetPayload = (object.writeSetPayload !== undefined && object.writeSetPayload !== null)
      ? WriteSetPayload.fromPartial(object.writeSetPayload)
      : undefined;
    message.multisigPayload = (object.multisigPayload !== undefined && object.multisigPayload !== null)
      ? MultisigPayload.fromPartial(object.multisigPayload)
      : undefined;
    message.extraConfigV1 = (object.extraConfigV1 !== undefined && object.extraConfigV1 !== null)
      ? ExtraConfigV1.fromPartial(object.extraConfigV1)
      : undefined;
    return message;
  },
};

function createBaseExtraConfigV1(): ExtraConfigV1 {
  return { multisigAddress: undefined, replayProtectionNonce: undefined };
}

export const ExtraConfigV1 = {
  encode(message: ExtraConfigV1, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.multisigAddress !== undefined) {
      writer.uint32(10).string(message.multisigAddress);
    }
    if (message.replayProtectionNonce !== undefined) {
      if (BigInt.asUintN(64, message.replayProtectionNonce) !== message.replayProtectionNonce) {
        throw new globalThis.Error("value provided for field message.replayProtectionNonce of type uint64 too large");
      }
      writer.uint32(16).uint64(message.replayProtectionNonce.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ExtraConfigV1 {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseExtraConfigV1();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.multisigAddress = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.replayProtectionNonce = longToBigint(reader.uint64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ExtraConfigV1, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ExtraConfigV1 | ExtraConfigV1[]> | Iterable<ExtraConfigV1 | ExtraConfigV1[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ExtraConfigV1.encode(p).finish()];
        }
      } else {
        yield* [ExtraConfigV1.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ExtraConfigV1>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ExtraConfigV1> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ExtraConfigV1.decode(p)];
        }
      } else {
        yield* [ExtraConfigV1.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ExtraConfigV1 {
    return {
      multisigAddress: isSet(object.multisigAddress) ? globalThis.String(object.multisigAddress) : undefined,
      replayProtectionNonce: isSet(object.replayProtectionNonce) ? BigInt(object.replayProtectionNonce) : undefined,
    };
  },

  toJSON(message: ExtraConfigV1): unknown {
    const obj: any = {};
    if (message.multisigAddress !== undefined) {
      obj.multisigAddress = message.multisigAddress;
    }
    if (message.replayProtectionNonce !== undefined) {
      obj.replayProtectionNonce = message.replayProtectionNonce.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<ExtraConfigV1>): ExtraConfigV1 {
    return ExtraConfigV1.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ExtraConfigV1>): ExtraConfigV1 {
    const message = createBaseExtraConfigV1();
    message.multisigAddress = object.multisigAddress ?? undefined;
    message.replayProtectionNonce = object.replayProtectionNonce ?? undefined;
    return message;
  },
};

function createBaseEntryFunctionPayload(): EntryFunctionPayload {
  return { function: undefined, typeArguments: [], arguments: [], entryFunctionIdStr: "" };
}

export const EntryFunctionPayload = {
  encode(message: EntryFunctionPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.function !== undefined) {
      EntryFunctionId.encode(message.function, writer.uint32(10).fork()).ldelim();
    }
    if (message.typeArguments !== undefined && message.typeArguments.length !== 0) {
      for (const v of message.typeArguments) {
        MoveType.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    if (message.arguments !== undefined && message.arguments.length !== 0) {
      for (const v of message.arguments) {
        writer.uint32(26).string(v!);
      }
    }
    if (message.entryFunctionIdStr !== undefined && message.entryFunctionIdStr !== "") {
      writer.uint32(34).string(message.entryFunctionIdStr);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EntryFunctionPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEntryFunctionPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.function = EntryFunctionId.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.typeArguments!.push(MoveType.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.arguments!.push(reader.string());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.entryFunctionIdStr = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<EntryFunctionPayload, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<EntryFunctionPayload | EntryFunctionPayload[]>
      | Iterable<EntryFunctionPayload | EntryFunctionPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionPayload.encode(p).finish()];
        }
      } else {
        yield* [EntryFunctionPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EntryFunctionPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EntryFunctionPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionPayload.decode(p)];
        }
      } else {
        yield* [EntryFunctionPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EntryFunctionPayload {
    return {
      function: isSet(object.function) ? EntryFunctionId.fromJSON(object.function) : undefined,
      typeArguments: globalThis.Array.isArray(object?.typeArguments)
        ? object.typeArguments.map((e: any) => MoveType.fromJSON(e))
        : [],
      arguments: globalThis.Array.isArray(object?.arguments)
        ? object.arguments.map((e: any) => globalThis.String(e))
        : [],
      entryFunctionIdStr: isSet(object.entryFunctionIdStr) ? globalThis.String(object.entryFunctionIdStr) : "",
    };
  },

  toJSON(message: EntryFunctionPayload): unknown {
    const obj: any = {};
    if (message.function !== undefined) {
      obj.function = EntryFunctionId.toJSON(message.function);
    }
    if (message.typeArguments?.length) {
      obj.typeArguments = message.typeArguments.map((e) => MoveType.toJSON(e));
    }
    if (message.arguments?.length) {
      obj.arguments = message.arguments;
    }
    if (message.entryFunctionIdStr !== undefined && message.entryFunctionIdStr !== "") {
      obj.entryFunctionIdStr = message.entryFunctionIdStr;
    }
    return obj;
  },

  create(base?: DeepPartial<EntryFunctionPayload>): EntryFunctionPayload {
    return EntryFunctionPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EntryFunctionPayload>): EntryFunctionPayload {
    const message = createBaseEntryFunctionPayload();
    message.function = (object.function !== undefined && object.function !== null)
      ? EntryFunctionId.fromPartial(object.function)
      : undefined;
    message.typeArguments = object.typeArguments?.map((e) => MoveType.fromPartial(e)) || [];
    message.arguments = object.arguments?.map((e) => e) || [];
    message.entryFunctionIdStr = object.entryFunctionIdStr ?? "";
    return message;
  },
};

function createBaseMoveScriptBytecode(): MoveScriptBytecode {
  return { bytecode: new Uint8Array(0), abi: undefined };
}

export const MoveScriptBytecode = {
  encode(message: MoveScriptBytecode, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.bytecode !== undefined && message.bytecode.length !== 0) {
      writer.uint32(10).bytes(message.bytecode);
    }
    if (message.abi !== undefined) {
      MoveFunction.encode(message.abi, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveScriptBytecode {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveScriptBytecode();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.bytecode = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.abi = MoveFunction.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveScriptBytecode, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveScriptBytecode | MoveScriptBytecode[]>
      | Iterable<MoveScriptBytecode | MoveScriptBytecode[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveScriptBytecode.encode(p).finish()];
        }
      } else {
        yield* [MoveScriptBytecode.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveScriptBytecode>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveScriptBytecode> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveScriptBytecode.decode(p)];
        }
      } else {
        yield* [MoveScriptBytecode.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveScriptBytecode {
    return {
      bytecode: isSet(object.bytecode) ? bytesFromBase64(object.bytecode) : new Uint8Array(0),
      abi: isSet(object.abi) ? MoveFunction.fromJSON(object.abi) : undefined,
    };
  },

  toJSON(message: MoveScriptBytecode): unknown {
    const obj: any = {};
    if (message.bytecode !== undefined && message.bytecode.length !== 0) {
      obj.bytecode = base64FromBytes(message.bytecode);
    }
    if (message.abi !== undefined) {
      obj.abi = MoveFunction.toJSON(message.abi);
    }
    return obj;
  },

  create(base?: DeepPartial<MoveScriptBytecode>): MoveScriptBytecode {
    return MoveScriptBytecode.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveScriptBytecode>): MoveScriptBytecode {
    const message = createBaseMoveScriptBytecode();
    message.bytecode = object.bytecode ?? new Uint8Array(0);
    message.abi = (object.abi !== undefined && object.abi !== null) ? MoveFunction.fromPartial(object.abi) : undefined;
    return message;
  },
};

function createBaseScriptPayload(): ScriptPayload {
  return { code: undefined, typeArguments: [], arguments: [] };
}

export const ScriptPayload = {
  encode(message: ScriptPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.code !== undefined) {
      MoveScriptBytecode.encode(message.code, writer.uint32(10).fork()).ldelim();
    }
    if (message.typeArguments !== undefined && message.typeArguments.length !== 0) {
      for (const v of message.typeArguments) {
        MoveType.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    if (message.arguments !== undefined && message.arguments.length !== 0) {
      for (const v of message.arguments) {
        writer.uint32(26).string(v!);
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ScriptPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseScriptPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.code = MoveScriptBytecode.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.typeArguments!.push(MoveType.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.arguments!.push(reader.string());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<ScriptPayload, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ScriptPayload | ScriptPayload[]> | Iterable<ScriptPayload | ScriptPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ScriptPayload.encode(p).finish()];
        }
      } else {
        yield* [ScriptPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ScriptPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ScriptPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ScriptPayload.decode(p)];
        }
      } else {
        yield* [ScriptPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ScriptPayload {
    return {
      code: isSet(object.code) ? MoveScriptBytecode.fromJSON(object.code) : undefined,
      typeArguments: globalThis.Array.isArray(object?.typeArguments)
        ? object.typeArguments.map((e: any) => MoveType.fromJSON(e))
        : [],
      arguments: globalThis.Array.isArray(object?.arguments)
        ? object.arguments.map((e: any) => globalThis.String(e))
        : [],
    };
  },

  toJSON(message: ScriptPayload): unknown {
    const obj: any = {};
    if (message.code !== undefined) {
      obj.code = MoveScriptBytecode.toJSON(message.code);
    }
    if (message.typeArguments?.length) {
      obj.typeArguments = message.typeArguments.map((e) => MoveType.toJSON(e));
    }
    if (message.arguments?.length) {
      obj.arguments = message.arguments;
    }
    return obj;
  },

  create(base?: DeepPartial<ScriptPayload>): ScriptPayload {
    return ScriptPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ScriptPayload>): ScriptPayload {
    const message = createBaseScriptPayload();
    message.code = (object.code !== undefined && object.code !== null)
      ? MoveScriptBytecode.fromPartial(object.code)
      : undefined;
    message.typeArguments = object.typeArguments?.map((e) => MoveType.fromPartial(e)) || [];
    message.arguments = object.arguments?.map((e) => e) || [];
    return message;
  },
};

function createBaseMultisigPayload(): MultisigPayload {
  return { multisigAddress: "", transactionPayload: undefined };
}

export const MultisigPayload = {
  encode(message: MultisigPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.multisigAddress !== undefined && message.multisigAddress !== "") {
      writer.uint32(10).string(message.multisigAddress);
    }
    if (message.transactionPayload !== undefined) {
      MultisigTransactionPayload.encode(message.transactionPayload, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MultisigPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMultisigPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.multisigAddress = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.transactionPayload = MultisigTransactionPayload.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MultisigPayload, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MultisigPayload | MultisigPayload[]> | Iterable<MultisigPayload | MultisigPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultisigPayload.encode(p).finish()];
        }
      } else {
        yield* [MultisigPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MultisigPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MultisigPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultisigPayload.decode(p)];
        }
      } else {
        yield* [MultisigPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MultisigPayload {
    return {
      multisigAddress: isSet(object.multisigAddress) ? globalThis.String(object.multisigAddress) : "",
      transactionPayload: isSet(object.transactionPayload)
        ? MultisigTransactionPayload.fromJSON(object.transactionPayload)
        : undefined,
    };
  },

  toJSON(message: MultisigPayload): unknown {
    const obj: any = {};
    if (message.multisigAddress !== undefined && message.multisigAddress !== "") {
      obj.multisigAddress = message.multisigAddress;
    }
    if (message.transactionPayload !== undefined) {
      obj.transactionPayload = MultisigTransactionPayload.toJSON(message.transactionPayload);
    }
    return obj;
  },

  create(base?: DeepPartial<MultisigPayload>): MultisigPayload {
    return MultisigPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MultisigPayload>): MultisigPayload {
    const message = createBaseMultisigPayload();
    message.multisigAddress = object.multisigAddress ?? "";
    message.transactionPayload = (object.transactionPayload !== undefined && object.transactionPayload !== null)
      ? MultisigTransactionPayload.fromPartial(object.transactionPayload)
      : undefined;
    return message;
  },
};

function createBaseMultisigTransactionPayload(): MultisigTransactionPayload {
  return { type: 0, entryFunctionPayload: undefined };
}

export const MultisigTransactionPayload = {
  encode(message: MultisigTransactionPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.entryFunctionPayload !== undefined) {
      EntryFunctionPayload.encode(message.entryFunctionPayload, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MultisigTransactionPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMultisigTransactionPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.entryFunctionPayload = EntryFunctionPayload.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MultisigTransactionPayload, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MultisigTransactionPayload | MultisigTransactionPayload[]>
      | Iterable<MultisigTransactionPayload | MultisigTransactionPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultisigTransactionPayload.encode(p).finish()];
        }
      } else {
        yield* [MultisigTransactionPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MultisigTransactionPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MultisigTransactionPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultisigTransactionPayload.decode(p)];
        }
      } else {
        yield* [MultisigTransactionPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MultisigTransactionPayload {
    return {
      type: isSet(object.type) ? multisigTransactionPayload_TypeFromJSON(object.type) : 0,
      entryFunctionPayload: isSet(object.entryFunctionPayload)
        ? EntryFunctionPayload.fromJSON(object.entryFunctionPayload)
        : undefined,
    };
  },

  toJSON(message: MultisigTransactionPayload): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = multisigTransactionPayload_TypeToJSON(message.type);
    }
    if (message.entryFunctionPayload !== undefined) {
      obj.entryFunctionPayload = EntryFunctionPayload.toJSON(message.entryFunctionPayload);
    }
    return obj;
  },

  create(base?: DeepPartial<MultisigTransactionPayload>): MultisigTransactionPayload {
    return MultisigTransactionPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MultisigTransactionPayload>): MultisigTransactionPayload {
    const message = createBaseMultisigTransactionPayload();
    message.type = object.type ?? 0;
    message.entryFunctionPayload = (object.entryFunctionPayload !== undefined && object.entryFunctionPayload !== null)
      ? EntryFunctionPayload.fromPartial(object.entryFunctionPayload)
      : undefined;
    return message;
  },
};

function createBaseMoveModuleBytecode(): MoveModuleBytecode {
  return { bytecode: new Uint8Array(0), abi: undefined };
}

export const MoveModuleBytecode = {
  encode(message: MoveModuleBytecode, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.bytecode !== undefined && message.bytecode.length !== 0) {
      writer.uint32(10).bytes(message.bytecode);
    }
    if (message.abi !== undefined) {
      MoveModule.encode(message.abi, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveModuleBytecode {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveModuleBytecode();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.bytecode = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.abi = MoveModule.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveModuleBytecode, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveModuleBytecode | MoveModuleBytecode[]>
      | Iterable<MoveModuleBytecode | MoveModuleBytecode[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModuleBytecode.encode(p).finish()];
        }
      } else {
        yield* [MoveModuleBytecode.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveModuleBytecode>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveModuleBytecode> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModuleBytecode.decode(p)];
        }
      } else {
        yield* [MoveModuleBytecode.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveModuleBytecode {
    return {
      bytecode: isSet(object.bytecode) ? bytesFromBase64(object.bytecode) : new Uint8Array(0),
      abi: isSet(object.abi) ? MoveModule.fromJSON(object.abi) : undefined,
    };
  },

  toJSON(message: MoveModuleBytecode): unknown {
    const obj: any = {};
    if (message.bytecode !== undefined && message.bytecode.length !== 0) {
      obj.bytecode = base64FromBytes(message.bytecode);
    }
    if (message.abi !== undefined) {
      obj.abi = MoveModule.toJSON(message.abi);
    }
    return obj;
  },

  create(base?: DeepPartial<MoveModuleBytecode>): MoveModuleBytecode {
    return MoveModuleBytecode.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveModuleBytecode>): MoveModuleBytecode {
    const message = createBaseMoveModuleBytecode();
    message.bytecode = object.bytecode ?? new Uint8Array(0);
    message.abi = (object.abi !== undefined && object.abi !== null) ? MoveModule.fromPartial(object.abi) : undefined;
    return message;
  },
};

function createBaseMoveModule(): MoveModule {
  return { address: "", name: "", friends: [], exposedFunctions: [], structs: [] };
}

export const MoveModule = {
  encode(message: MoveModule, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(18).string(message.name);
    }
    if (message.friends !== undefined && message.friends.length !== 0) {
      for (const v of message.friends) {
        MoveModuleId.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    if (message.exposedFunctions !== undefined && message.exposedFunctions.length !== 0) {
      for (const v of message.exposedFunctions) {
        MoveFunction.encode(v!, writer.uint32(34).fork()).ldelim();
      }
    }
    if (message.structs !== undefined && message.structs.length !== 0) {
      for (const v of message.structs) {
        MoveStruct.encode(v!, writer.uint32(42).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveModule {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveModule();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.name = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.friends!.push(MoveModuleId.decode(reader, reader.uint32()));
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.exposedFunctions!.push(MoveFunction.decode(reader, reader.uint32()));
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.structs!.push(MoveStruct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveModule, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveModule | MoveModule[]> | Iterable<MoveModule | MoveModule[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModule.encode(p).finish()];
        }
      } else {
        yield* [MoveModule.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveModule>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveModule> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModule.decode(p)];
        }
      } else {
        yield* [MoveModule.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveModule {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      friends: globalThis.Array.isArray(object?.friends)
        ? object.friends.map((e: any) => MoveModuleId.fromJSON(e))
        : [],
      exposedFunctions: globalThis.Array.isArray(object?.exposedFunctions)
        ? object.exposedFunctions.map((e: any) => MoveFunction.fromJSON(e))
        : [],
      structs: globalThis.Array.isArray(object?.structs) ? object.structs.map((e: any) => MoveStruct.fromJSON(e)) : [],
    };
  },

  toJSON(message: MoveModule): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    if (message.friends?.length) {
      obj.friends = message.friends.map((e) => MoveModuleId.toJSON(e));
    }
    if (message.exposedFunctions?.length) {
      obj.exposedFunctions = message.exposedFunctions.map((e) => MoveFunction.toJSON(e));
    }
    if (message.structs?.length) {
      obj.structs = message.structs.map((e) => MoveStruct.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MoveModule>): MoveModule {
    return MoveModule.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveModule>): MoveModule {
    const message = createBaseMoveModule();
    message.address = object.address ?? "";
    message.name = object.name ?? "";
    message.friends = object.friends?.map((e) => MoveModuleId.fromPartial(e)) || [];
    message.exposedFunctions = object.exposedFunctions?.map((e) => MoveFunction.fromPartial(e)) || [];
    message.structs = object.structs?.map((e) => MoveStruct.fromPartial(e)) || [];
    return message;
  },
};

function createBaseMoveFunction(): MoveFunction {
  return { name: "", visibility: 0, isEntry: false, genericTypeParams: [], params: [], return: [] };
}

export const MoveFunction = {
  encode(message: MoveFunction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(10).string(message.name);
    }
    if (message.visibility !== undefined && message.visibility !== 0) {
      writer.uint32(16).int32(message.visibility);
    }
    if (message.isEntry === true) {
      writer.uint32(24).bool(message.isEntry);
    }
    if (message.genericTypeParams !== undefined && message.genericTypeParams.length !== 0) {
      for (const v of message.genericTypeParams) {
        MoveFunctionGenericTypeParam.encode(v!, writer.uint32(34).fork()).ldelim();
      }
    }
    if (message.params !== undefined && message.params.length !== 0) {
      for (const v of message.params) {
        MoveType.encode(v!, writer.uint32(42).fork()).ldelim();
      }
    }
    if (message.return !== undefined && message.return.length !== 0) {
      for (const v of message.return) {
        MoveType.encode(v!, writer.uint32(50).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveFunction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveFunction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.name = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.visibility = reader.int32() as any;
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.isEntry = reader.bool();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.genericTypeParams!.push(MoveFunctionGenericTypeParam.decode(reader, reader.uint32()));
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.params!.push(MoveType.decode(reader, reader.uint32()));
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.return!.push(MoveType.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveFunction, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveFunction | MoveFunction[]> | Iterable<MoveFunction | MoveFunction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveFunction.encode(p).finish()];
        }
      } else {
        yield* [MoveFunction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveFunction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveFunction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveFunction.decode(p)];
        }
      } else {
        yield* [MoveFunction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveFunction {
    return {
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      visibility: isSet(object.visibility) ? moveFunction_VisibilityFromJSON(object.visibility) : 0,
      isEntry: isSet(object.isEntry) ? globalThis.Boolean(object.isEntry) : false,
      genericTypeParams: globalThis.Array.isArray(object?.genericTypeParams)
        ? object.genericTypeParams.map((e: any) => MoveFunctionGenericTypeParam.fromJSON(e))
        : [],
      params: globalThis.Array.isArray(object?.params) ? object.params.map((e: any) => MoveType.fromJSON(e)) : [],
      return: globalThis.Array.isArray(object?.return) ? object.return.map((e: any) => MoveType.fromJSON(e)) : [],
    };
  },

  toJSON(message: MoveFunction): unknown {
    const obj: any = {};
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    if (message.visibility !== undefined && message.visibility !== 0) {
      obj.visibility = moveFunction_VisibilityToJSON(message.visibility);
    }
    if (message.isEntry === true) {
      obj.isEntry = message.isEntry;
    }
    if (message.genericTypeParams?.length) {
      obj.genericTypeParams = message.genericTypeParams.map((e) => MoveFunctionGenericTypeParam.toJSON(e));
    }
    if (message.params?.length) {
      obj.params = message.params.map((e) => MoveType.toJSON(e));
    }
    if (message.return?.length) {
      obj.return = message.return.map((e) => MoveType.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MoveFunction>): MoveFunction {
    return MoveFunction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveFunction>): MoveFunction {
    const message = createBaseMoveFunction();
    message.name = object.name ?? "";
    message.visibility = object.visibility ?? 0;
    message.isEntry = object.isEntry ?? false;
    message.genericTypeParams = object.genericTypeParams?.map((e) => MoveFunctionGenericTypeParam.fromPartial(e)) || [];
    message.params = object.params?.map((e) => MoveType.fromPartial(e)) || [];
    message.return = object.return?.map((e) => MoveType.fromPartial(e)) || [];
    return message;
  },
};

function createBaseMoveStruct(): MoveStruct {
  return { name: "", isNative: false, isEvent: false, abilities: [], genericTypeParams: [], fields: [] };
}

export const MoveStruct = {
  encode(message: MoveStruct, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(10).string(message.name);
    }
    if (message.isNative === true) {
      writer.uint32(16).bool(message.isNative);
    }
    if (message.isEvent === true) {
      writer.uint32(48).bool(message.isEvent);
    }
    if (message.abilities !== undefined && message.abilities.length !== 0) {
      writer.uint32(26).fork();
      for (const v of message.abilities) {
        writer.int32(v);
      }
      writer.ldelim();
    }
    if (message.genericTypeParams !== undefined && message.genericTypeParams.length !== 0) {
      for (const v of message.genericTypeParams) {
        MoveStructGenericTypeParam.encode(v!, writer.uint32(34).fork()).ldelim();
      }
    }
    if (message.fields !== undefined && message.fields.length !== 0) {
      for (const v of message.fields) {
        MoveStructField.encode(v!, writer.uint32(42).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveStruct {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveStruct();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.name = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.isNative = reader.bool();
          continue;
        case 6:
          if (tag !== 48) {
            break;
          }

          message.isEvent = reader.bool();
          continue;
        case 3:
          if (tag === 24) {
            message.abilities!.push(reader.int32() as any);

            continue;
          }

          if (tag === 26) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.abilities!.push(reader.int32() as any);
            }

            continue;
          }

          break;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.genericTypeParams!.push(MoveStructGenericTypeParam.decode(reader, reader.uint32()));
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.fields!.push(MoveStructField.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveStruct, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveStruct | MoveStruct[]> | Iterable<MoveStruct | MoveStruct[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStruct.encode(p).finish()];
        }
      } else {
        yield* [MoveStruct.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveStruct>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveStruct> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStruct.decode(p)];
        }
      } else {
        yield* [MoveStruct.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveStruct {
    return {
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      isNative: isSet(object.isNative) ? globalThis.Boolean(object.isNative) : false,
      isEvent: isSet(object.isEvent) ? globalThis.Boolean(object.isEvent) : false,
      abilities: globalThis.Array.isArray(object?.abilities)
        ? object.abilities.map((e: any) => moveAbilityFromJSON(e))
        : [],
      genericTypeParams: globalThis.Array.isArray(object?.genericTypeParams)
        ? object.genericTypeParams.map((e: any) => MoveStructGenericTypeParam.fromJSON(e))
        : [],
      fields: globalThis.Array.isArray(object?.fields)
        ? object.fields.map((e: any) => MoveStructField.fromJSON(e))
        : [],
    };
  },

  toJSON(message: MoveStruct): unknown {
    const obj: any = {};
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    if (message.isNative === true) {
      obj.isNative = message.isNative;
    }
    if (message.isEvent === true) {
      obj.isEvent = message.isEvent;
    }
    if (message.abilities?.length) {
      obj.abilities = message.abilities.map((e) => moveAbilityToJSON(e));
    }
    if (message.genericTypeParams?.length) {
      obj.genericTypeParams = message.genericTypeParams.map((e) => MoveStructGenericTypeParam.toJSON(e));
    }
    if (message.fields?.length) {
      obj.fields = message.fields.map((e) => MoveStructField.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MoveStruct>): MoveStruct {
    return MoveStruct.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveStruct>): MoveStruct {
    const message = createBaseMoveStruct();
    message.name = object.name ?? "";
    message.isNative = object.isNative ?? false;
    message.isEvent = object.isEvent ?? false;
    message.abilities = object.abilities?.map((e) => e) || [];
    message.genericTypeParams = object.genericTypeParams?.map((e) => MoveStructGenericTypeParam.fromPartial(e)) || [];
    message.fields = object.fields?.map((e) => MoveStructField.fromPartial(e)) || [];
    return message;
  },
};

function createBaseMoveStructGenericTypeParam(): MoveStructGenericTypeParam {
  return { constraints: [], isPhantom: false };
}

export const MoveStructGenericTypeParam = {
  encode(message: MoveStructGenericTypeParam, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.constraints !== undefined && message.constraints.length !== 0) {
      writer.uint32(10).fork();
      for (const v of message.constraints) {
        writer.int32(v);
      }
      writer.ldelim();
    }
    if (message.isPhantom === true) {
      writer.uint32(16).bool(message.isPhantom);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveStructGenericTypeParam {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveStructGenericTypeParam();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag === 8) {
            message.constraints!.push(reader.int32() as any);

            continue;
          }

          if (tag === 10) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.constraints!.push(reader.int32() as any);
            }

            continue;
          }

          break;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.isPhantom = reader.bool();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveStructGenericTypeParam, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveStructGenericTypeParam | MoveStructGenericTypeParam[]>
      | Iterable<MoveStructGenericTypeParam | MoveStructGenericTypeParam[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructGenericTypeParam.encode(p).finish()];
        }
      } else {
        yield* [MoveStructGenericTypeParam.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveStructGenericTypeParam>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveStructGenericTypeParam> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructGenericTypeParam.decode(p)];
        }
      } else {
        yield* [MoveStructGenericTypeParam.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveStructGenericTypeParam {
    return {
      constraints: globalThis.Array.isArray(object?.constraints)
        ? object.constraints.map((e: any) => moveAbilityFromJSON(e))
        : [],
      isPhantom: isSet(object.isPhantom) ? globalThis.Boolean(object.isPhantom) : false,
    };
  },

  toJSON(message: MoveStructGenericTypeParam): unknown {
    const obj: any = {};
    if (message.constraints?.length) {
      obj.constraints = message.constraints.map((e) => moveAbilityToJSON(e));
    }
    if (message.isPhantom === true) {
      obj.isPhantom = message.isPhantom;
    }
    return obj;
  },

  create(base?: DeepPartial<MoveStructGenericTypeParam>): MoveStructGenericTypeParam {
    return MoveStructGenericTypeParam.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveStructGenericTypeParam>): MoveStructGenericTypeParam {
    const message = createBaseMoveStructGenericTypeParam();
    message.constraints = object.constraints?.map((e) => e) || [];
    message.isPhantom = object.isPhantom ?? false;
    return message;
  },
};

function createBaseMoveStructField(): MoveStructField {
  return { name: "", type: undefined };
}

export const MoveStructField = {
  encode(message: MoveStructField, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(10).string(message.name);
    }
    if (message.type !== undefined) {
      MoveType.encode(message.type, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveStructField {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveStructField();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.name = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.type = MoveType.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveStructField, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveStructField | MoveStructField[]> | Iterable<MoveStructField | MoveStructField[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructField.encode(p).finish()];
        }
      } else {
        yield* [MoveStructField.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveStructField>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveStructField> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructField.decode(p)];
        }
      } else {
        yield* [MoveStructField.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveStructField {
    return {
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      type: isSet(object.type) ? MoveType.fromJSON(object.type) : undefined,
    };
  },

  toJSON(message: MoveStructField): unknown {
    const obj: any = {};
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    if (message.type !== undefined) {
      obj.type = MoveType.toJSON(message.type);
    }
    return obj;
  },

  create(base?: DeepPartial<MoveStructField>): MoveStructField {
    return MoveStructField.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveStructField>): MoveStructField {
    const message = createBaseMoveStructField();
    message.name = object.name ?? "";
    message.type = (object.type !== undefined && object.type !== null) ? MoveType.fromPartial(object.type) : undefined;
    return message;
  },
};

function createBaseMoveFunctionGenericTypeParam(): MoveFunctionGenericTypeParam {
  return { constraints: [] };
}

export const MoveFunctionGenericTypeParam = {
  encode(message: MoveFunctionGenericTypeParam, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.constraints !== undefined && message.constraints.length !== 0) {
      writer.uint32(10).fork();
      for (const v of message.constraints) {
        writer.int32(v);
      }
      writer.ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveFunctionGenericTypeParam {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveFunctionGenericTypeParam();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag === 8) {
            message.constraints!.push(reader.int32() as any);

            continue;
          }

          if (tag === 10) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.constraints!.push(reader.int32() as any);
            }

            continue;
          }

          break;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveFunctionGenericTypeParam, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveFunctionGenericTypeParam | MoveFunctionGenericTypeParam[]>
      | Iterable<MoveFunctionGenericTypeParam | MoveFunctionGenericTypeParam[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveFunctionGenericTypeParam.encode(p).finish()];
        }
      } else {
        yield* [MoveFunctionGenericTypeParam.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveFunctionGenericTypeParam>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveFunctionGenericTypeParam> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveFunctionGenericTypeParam.decode(p)];
        }
      } else {
        yield* [MoveFunctionGenericTypeParam.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveFunctionGenericTypeParam {
    return {
      constraints: globalThis.Array.isArray(object?.constraints)
        ? object.constraints.map((e: any) => moveAbilityFromJSON(e))
        : [],
    };
  },

  toJSON(message: MoveFunctionGenericTypeParam): unknown {
    const obj: any = {};
    if (message.constraints?.length) {
      obj.constraints = message.constraints.map((e) => moveAbilityToJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MoveFunctionGenericTypeParam>): MoveFunctionGenericTypeParam {
    return MoveFunctionGenericTypeParam.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveFunctionGenericTypeParam>): MoveFunctionGenericTypeParam {
    const message = createBaseMoveFunctionGenericTypeParam();
    message.constraints = object.constraints?.map((e) => e) || [];
    return message;
  },
};

function createBaseMoveType(): MoveType {
  return {
    type: 0,
    vector: undefined,
    struct: undefined,
    genericTypeParamIndex: undefined,
    reference: undefined,
    unparsable: undefined,
  };
}

export const MoveType = {
  encode(message: MoveType, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.vector !== undefined) {
      MoveType.encode(message.vector, writer.uint32(26).fork()).ldelim();
    }
    if (message.struct !== undefined) {
      MoveStructTag.encode(message.struct, writer.uint32(34).fork()).ldelim();
    }
    if (message.genericTypeParamIndex !== undefined) {
      writer.uint32(40).uint32(message.genericTypeParamIndex);
    }
    if (message.reference !== undefined) {
      MoveType_ReferenceType.encode(message.reference, writer.uint32(50).fork()).ldelim();
    }
    if (message.unparsable !== undefined) {
      writer.uint32(58).string(message.unparsable);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveType {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveType();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.vector = MoveType.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.struct = MoveStructTag.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.genericTypeParamIndex = reader.uint32();
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.reference = MoveType_ReferenceType.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.unparsable = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveType, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveType | MoveType[]> | Iterable<MoveType | MoveType[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveType.encode(p).finish()];
        }
      } else {
        yield* [MoveType.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveType>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveType> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveType.decode(p)];
        }
      } else {
        yield* [MoveType.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveType {
    return {
      type: isSet(object.type) ? moveTypesFromJSON(object.type) : 0,
      vector: isSet(object.vector) ? MoveType.fromJSON(object.vector) : undefined,
      struct: isSet(object.struct) ? MoveStructTag.fromJSON(object.struct) : undefined,
      genericTypeParamIndex: isSet(object.genericTypeParamIndex)
        ? globalThis.Number(object.genericTypeParamIndex)
        : undefined,
      reference: isSet(object.reference) ? MoveType_ReferenceType.fromJSON(object.reference) : undefined,
      unparsable: isSet(object.unparsable) ? globalThis.String(object.unparsable) : undefined,
    };
  },

  toJSON(message: MoveType): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = moveTypesToJSON(message.type);
    }
    if (message.vector !== undefined) {
      obj.vector = MoveType.toJSON(message.vector);
    }
    if (message.struct !== undefined) {
      obj.struct = MoveStructTag.toJSON(message.struct);
    }
    if (message.genericTypeParamIndex !== undefined) {
      obj.genericTypeParamIndex = Math.round(message.genericTypeParamIndex);
    }
    if (message.reference !== undefined) {
      obj.reference = MoveType_ReferenceType.toJSON(message.reference);
    }
    if (message.unparsable !== undefined) {
      obj.unparsable = message.unparsable;
    }
    return obj;
  },

  create(base?: DeepPartial<MoveType>): MoveType {
    return MoveType.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveType>): MoveType {
    const message = createBaseMoveType();
    message.type = object.type ?? 0;
    message.vector = (object.vector !== undefined && object.vector !== null)
      ? MoveType.fromPartial(object.vector)
      : undefined;
    message.struct = (object.struct !== undefined && object.struct !== null)
      ? MoveStructTag.fromPartial(object.struct)
      : undefined;
    message.genericTypeParamIndex = object.genericTypeParamIndex ?? undefined;
    message.reference = (object.reference !== undefined && object.reference !== null)
      ? MoveType_ReferenceType.fromPartial(object.reference)
      : undefined;
    message.unparsable = object.unparsable ?? undefined;
    return message;
  },
};

function createBaseMoveType_ReferenceType(): MoveType_ReferenceType {
  return { mutable: false, to: undefined };
}

export const MoveType_ReferenceType = {
  encode(message: MoveType_ReferenceType, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.mutable === true) {
      writer.uint32(8).bool(message.mutable);
    }
    if (message.to !== undefined) {
      MoveType.encode(message.to, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveType_ReferenceType {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveType_ReferenceType();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.mutable = reader.bool();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.to = MoveType.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveType_ReferenceType, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveType_ReferenceType | MoveType_ReferenceType[]>
      | Iterable<MoveType_ReferenceType | MoveType_ReferenceType[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveType_ReferenceType.encode(p).finish()];
        }
      } else {
        yield* [MoveType_ReferenceType.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveType_ReferenceType>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveType_ReferenceType> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveType_ReferenceType.decode(p)];
        }
      } else {
        yield* [MoveType_ReferenceType.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveType_ReferenceType {
    return {
      mutable: isSet(object.mutable) ? globalThis.Boolean(object.mutable) : false,
      to: isSet(object.to) ? MoveType.fromJSON(object.to) : undefined,
    };
  },

  toJSON(message: MoveType_ReferenceType): unknown {
    const obj: any = {};
    if (message.mutable === true) {
      obj.mutable = message.mutable;
    }
    if (message.to !== undefined) {
      obj.to = MoveType.toJSON(message.to);
    }
    return obj;
  },

  create(base?: DeepPartial<MoveType_ReferenceType>): MoveType_ReferenceType {
    return MoveType_ReferenceType.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveType_ReferenceType>): MoveType_ReferenceType {
    const message = createBaseMoveType_ReferenceType();
    message.mutable = object.mutable ?? false;
    message.to = (object.to !== undefined && object.to !== null) ? MoveType.fromPartial(object.to) : undefined;
    return message;
  },
};

function createBaseWriteSetPayload(): WriteSetPayload {
  return { writeSet: undefined };
}

export const WriteSetPayload = {
  encode(message: WriteSetPayload, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.writeSet !== undefined) {
      WriteSet.encode(message.writeSet, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteSetPayload {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteSetPayload();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.writeSet = WriteSet.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteSetPayload, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteSetPayload | WriteSetPayload[]> | Iterable<WriteSetPayload | WriteSetPayload[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSetPayload.encode(p).finish()];
        }
      } else {
        yield* [WriteSetPayload.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteSetPayload>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteSetPayload> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteSetPayload.decode(p)];
        }
      } else {
        yield* [WriteSetPayload.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteSetPayload {
    return { writeSet: isSet(object.writeSet) ? WriteSet.fromJSON(object.writeSet) : undefined };
  },

  toJSON(message: WriteSetPayload): unknown {
    const obj: any = {};
    if (message.writeSet !== undefined) {
      obj.writeSet = WriteSet.toJSON(message.writeSet);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteSetPayload>): WriteSetPayload {
    return WriteSetPayload.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteSetPayload>): WriteSetPayload {
    const message = createBaseWriteSetPayload();
    message.writeSet = (object.writeSet !== undefined && object.writeSet !== null)
      ? WriteSet.fromPartial(object.writeSet)
      : undefined;
    return message;
  },
};

function createBaseEntryFunctionId(): EntryFunctionId {
  return { module: undefined, name: "" };
}

export const EntryFunctionId = {
  encode(message: EntryFunctionId, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.module !== undefined) {
      MoveModuleId.encode(message.module, writer.uint32(10).fork()).ldelim();
    }
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(18).string(message.name);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EntryFunctionId {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEntryFunctionId();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.module = MoveModuleId.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.name = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<EntryFunctionId, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EntryFunctionId | EntryFunctionId[]> | Iterable<EntryFunctionId | EntryFunctionId[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionId.encode(p).finish()];
        }
      } else {
        yield* [EntryFunctionId.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EntryFunctionId>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EntryFunctionId> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionId.decode(p)];
        }
      } else {
        yield* [EntryFunctionId.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EntryFunctionId {
    return {
      module: isSet(object.module) ? MoveModuleId.fromJSON(object.module) : undefined,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
    };
  },

  toJSON(message: EntryFunctionId): unknown {
    const obj: any = {};
    if (message.module !== undefined) {
      obj.module = MoveModuleId.toJSON(message.module);
    }
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    return obj;
  },

  create(base?: DeepPartial<EntryFunctionId>): EntryFunctionId {
    return EntryFunctionId.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EntryFunctionId>): EntryFunctionId {
    const message = createBaseEntryFunctionId();
    message.module = (object.module !== undefined && object.module !== null)
      ? MoveModuleId.fromPartial(object.module)
      : undefined;
    message.name = object.name ?? "";
    return message;
  },
};

function createBaseMoveModuleId(): MoveModuleId {
  return { address: "", name: "" };
}

export const MoveModuleId = {
  encode(message: MoveModuleId, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(18).string(message.name);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveModuleId {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveModuleId();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.name = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveModuleId, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveModuleId | MoveModuleId[]> | Iterable<MoveModuleId | MoveModuleId[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModuleId.encode(p).finish()];
        }
      } else {
        yield* [MoveModuleId.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveModuleId>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveModuleId> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveModuleId.decode(p)];
        }
      } else {
        yield* [MoveModuleId.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveModuleId {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      name: isSet(object.name) ? globalThis.String(object.name) : "",
    };
  },

  toJSON(message: MoveModuleId): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    return obj;
  },

  create(base?: DeepPartial<MoveModuleId>): MoveModuleId {
    return MoveModuleId.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveModuleId>): MoveModuleId {
    const message = createBaseMoveModuleId();
    message.address = object.address ?? "";
    message.name = object.name ?? "";
    return message;
  },
};

function createBaseMoveStructTag(): MoveStructTag {
  return { address: "", module: "", name: "", genericTypeParams: [] };
}

export const MoveStructTag = {
  encode(message: MoveStructTag, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined && message.address !== "") {
      writer.uint32(10).string(message.address);
    }
    if (message.module !== undefined && message.module !== "") {
      writer.uint32(18).string(message.module);
    }
    if (message.name !== undefined && message.name !== "") {
      writer.uint32(26).string(message.name);
    }
    if (message.genericTypeParams !== undefined && message.genericTypeParams.length !== 0) {
      for (const v of message.genericTypeParams) {
        MoveType.encode(v!, writer.uint32(34).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveStructTag {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveStructTag();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.module = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.name = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.genericTypeParams!.push(MoveType.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MoveStructTag, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MoveStructTag | MoveStructTag[]> | Iterable<MoveStructTag | MoveStructTag[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructTag.encode(p).finish()];
        }
      } else {
        yield* [MoveStructTag.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveStructTag>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveStructTag> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructTag.decode(p)];
        }
      } else {
        yield* [MoveStructTag.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveStructTag {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : "",
      module: isSet(object.module) ? globalThis.String(object.module) : "",
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      genericTypeParams: globalThis.Array.isArray(object?.genericTypeParams)
        ? object.genericTypeParams.map((e: any) => MoveType.fromJSON(e))
        : [],
    };
  },

  toJSON(message: MoveStructTag): unknown {
    const obj: any = {};
    if (message.address !== undefined && message.address !== "") {
      obj.address = message.address;
    }
    if (message.module !== undefined && message.module !== "") {
      obj.module = message.module;
    }
    if (message.name !== undefined && message.name !== "") {
      obj.name = message.name;
    }
    if (message.genericTypeParams?.length) {
      obj.genericTypeParams = message.genericTypeParams.map((e) => MoveType.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MoveStructTag>): MoveStructTag {
    return MoveStructTag.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveStructTag>): MoveStructTag {
    const message = createBaseMoveStructTag();
    message.address = object.address ?? "";
    message.module = object.module ?? "";
    message.name = object.name ?? "";
    message.genericTypeParams = object.genericTypeParams?.map((e) => MoveType.fromPartial(e)) || [];
    return message;
  },
};

function createBaseSignature(): Signature {
  return {
    type: 0,
    ed25519: undefined,
    multiEd25519: undefined,
    multiAgent: undefined,
    feePayer: undefined,
    singleSender: undefined,
  };
}

export const Signature = {
  encode(message: Signature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.ed25519 !== undefined) {
      Ed25519Signature.encode(message.ed25519, writer.uint32(18).fork()).ldelim();
    }
    if (message.multiEd25519 !== undefined) {
      MultiEd25519Signature.encode(message.multiEd25519, writer.uint32(26).fork()).ldelim();
    }
    if (message.multiAgent !== undefined) {
      MultiAgentSignature.encode(message.multiAgent, writer.uint32(34).fork()).ldelim();
    }
    if (message.feePayer !== undefined) {
      FeePayerSignature.encode(message.feePayer, writer.uint32(42).fork()).ldelim();
    }
    if (message.singleSender !== undefined) {
      SingleSender.encode(message.singleSender, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Signature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ed25519 = Ed25519Signature.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.multiEd25519 = MultiEd25519Signature.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.multiAgent = MultiAgentSignature.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.feePayer = FeePayerSignature.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.singleSender = SingleSender.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Signature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Signature | Signature[]> | Iterable<Signature | Signature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Signature.encode(p).finish()];
        }
      } else {
        yield* [Signature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Signature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Signature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Signature.decode(p)];
        }
      } else {
        yield* [Signature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Signature {
    return {
      type: isSet(object.type) ? signature_TypeFromJSON(object.type) : 0,
      ed25519: isSet(object.ed25519) ? Ed25519Signature.fromJSON(object.ed25519) : undefined,
      multiEd25519: isSet(object.multiEd25519) ? MultiEd25519Signature.fromJSON(object.multiEd25519) : undefined,
      multiAgent: isSet(object.multiAgent) ? MultiAgentSignature.fromJSON(object.multiAgent) : undefined,
      feePayer: isSet(object.feePayer) ? FeePayerSignature.fromJSON(object.feePayer) : undefined,
      singleSender: isSet(object.singleSender) ? SingleSender.fromJSON(object.singleSender) : undefined,
    };
  },

  toJSON(message: Signature): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = signature_TypeToJSON(message.type);
    }
    if (message.ed25519 !== undefined) {
      obj.ed25519 = Ed25519Signature.toJSON(message.ed25519);
    }
    if (message.multiEd25519 !== undefined) {
      obj.multiEd25519 = MultiEd25519Signature.toJSON(message.multiEd25519);
    }
    if (message.multiAgent !== undefined) {
      obj.multiAgent = MultiAgentSignature.toJSON(message.multiAgent);
    }
    if (message.feePayer !== undefined) {
      obj.feePayer = FeePayerSignature.toJSON(message.feePayer);
    }
    if (message.singleSender !== undefined) {
      obj.singleSender = SingleSender.toJSON(message.singleSender);
    }
    return obj;
  },

  create(base?: DeepPartial<Signature>): Signature {
    return Signature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Signature>): Signature {
    const message = createBaseSignature();
    message.type = object.type ?? 0;
    message.ed25519 = (object.ed25519 !== undefined && object.ed25519 !== null)
      ? Ed25519Signature.fromPartial(object.ed25519)
      : undefined;
    message.multiEd25519 = (object.multiEd25519 !== undefined && object.multiEd25519 !== null)
      ? MultiEd25519Signature.fromPartial(object.multiEd25519)
      : undefined;
    message.multiAgent = (object.multiAgent !== undefined && object.multiAgent !== null)
      ? MultiAgentSignature.fromPartial(object.multiAgent)
      : undefined;
    message.feePayer = (object.feePayer !== undefined && object.feePayer !== null)
      ? FeePayerSignature.fromPartial(object.feePayer)
      : undefined;
    message.singleSender = (object.singleSender !== undefined && object.singleSender !== null)
      ? SingleSender.fromPartial(object.singleSender)
      : undefined;
    return message;
  },
};

function createBaseEd25519Signature(): Ed25519Signature {
  return { publicKey: new Uint8Array(0), signature: new Uint8Array(0) };
}

export const Ed25519Signature = {
  encode(message: Ed25519Signature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.publicKey !== undefined && message.publicKey.length !== 0) {
      writer.uint32(10).bytes(message.publicKey);
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(18).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Ed25519Signature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEd25519Signature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.publicKey = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Ed25519Signature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Ed25519Signature | Ed25519Signature[]> | Iterable<Ed25519Signature | Ed25519Signature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Ed25519Signature.encode(p).finish()];
        }
      } else {
        yield* [Ed25519Signature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Ed25519Signature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Ed25519Signature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Ed25519Signature.decode(p)];
        }
      } else {
        yield* [Ed25519Signature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Ed25519Signature {
    return {
      publicKey: isSet(object.publicKey) ? bytesFromBase64(object.publicKey) : new Uint8Array(0),
      signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0),
    };
  },

  toJSON(message: Ed25519Signature): unknown {
    const obj: any = {};
    if (message.publicKey !== undefined && message.publicKey.length !== 0) {
      obj.publicKey = base64FromBytes(message.publicKey);
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<Ed25519Signature>): Ed25519Signature {
    return Ed25519Signature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Ed25519Signature>): Ed25519Signature {
    const message = createBaseEd25519Signature();
    message.publicKey = object.publicKey ?? new Uint8Array(0);
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseMultiEd25519Signature(): MultiEd25519Signature {
  return { publicKeys: [], signatures: [], threshold: 0, publicKeyIndices: [] };
}

export const MultiEd25519Signature = {
  encode(message: MultiEd25519Signature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.publicKeys !== undefined && message.publicKeys.length !== 0) {
      for (const v of message.publicKeys) {
        writer.uint32(10).bytes(v!);
      }
    }
    if (message.signatures !== undefined && message.signatures.length !== 0) {
      for (const v of message.signatures) {
        writer.uint32(18).bytes(v!);
      }
    }
    if (message.threshold !== undefined && message.threshold !== 0) {
      writer.uint32(24).uint32(message.threshold);
    }
    if (message.publicKeyIndices !== undefined && message.publicKeyIndices.length !== 0) {
      writer.uint32(34).fork();
      for (const v of message.publicKeyIndices) {
        writer.uint32(v);
      }
      writer.ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MultiEd25519Signature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMultiEd25519Signature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.publicKeys!.push(reader.bytes());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signatures!.push(reader.bytes());
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.threshold = reader.uint32();
          continue;
        case 4:
          if (tag === 32) {
            message.publicKeyIndices!.push(reader.uint32());

            continue;
          }

          if (tag === 34) {
            const end2 = reader.uint32() + reader.pos;
            while (reader.pos < end2) {
              message.publicKeyIndices!.push(reader.uint32());
            }

            continue;
          }

          break;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MultiEd25519Signature, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MultiEd25519Signature | MultiEd25519Signature[]>
      | Iterable<MultiEd25519Signature | MultiEd25519Signature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiEd25519Signature.encode(p).finish()];
        }
      } else {
        yield* [MultiEd25519Signature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MultiEd25519Signature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MultiEd25519Signature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiEd25519Signature.decode(p)];
        }
      } else {
        yield* [MultiEd25519Signature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MultiEd25519Signature {
    return {
      publicKeys: globalThis.Array.isArray(object?.publicKeys)
        ? object.publicKeys.map((e: any) => bytesFromBase64(e))
        : [],
      signatures: globalThis.Array.isArray(object?.signatures)
        ? object.signatures.map((e: any) => bytesFromBase64(e))
        : [],
      threshold: isSet(object.threshold) ? globalThis.Number(object.threshold) : 0,
      publicKeyIndices: globalThis.Array.isArray(object?.publicKeyIndices)
        ? object.publicKeyIndices.map((e: any) => globalThis.Number(e))
        : [],
    };
  },

  toJSON(message: MultiEd25519Signature): unknown {
    const obj: any = {};
    if (message.publicKeys?.length) {
      obj.publicKeys = message.publicKeys.map((e) => base64FromBytes(e));
    }
    if (message.signatures?.length) {
      obj.signatures = message.signatures.map((e) => base64FromBytes(e));
    }
    if (message.threshold !== undefined && message.threshold !== 0) {
      obj.threshold = Math.round(message.threshold);
    }
    if (message.publicKeyIndices?.length) {
      obj.publicKeyIndices = message.publicKeyIndices.map((e) => Math.round(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MultiEd25519Signature>): MultiEd25519Signature {
    return MultiEd25519Signature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MultiEd25519Signature>): MultiEd25519Signature {
    const message = createBaseMultiEd25519Signature();
    message.publicKeys = object.publicKeys?.map((e) => e) || [];
    message.signatures = object.signatures?.map((e) => e) || [];
    message.threshold = object.threshold ?? 0;
    message.publicKeyIndices = object.publicKeyIndices?.map((e) => e) || [];
    return message;
  },
};

function createBaseMultiAgentSignature(): MultiAgentSignature {
  return { sender: undefined, secondarySignerAddresses: [], secondarySigners: [] };
}

export const MultiAgentSignature = {
  encode(message: MultiAgentSignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sender !== undefined) {
      AccountSignature.encode(message.sender, writer.uint32(10).fork()).ldelim();
    }
    if (message.secondarySignerAddresses !== undefined && message.secondarySignerAddresses.length !== 0) {
      for (const v of message.secondarySignerAddresses) {
        writer.uint32(18).string(v!);
      }
    }
    if (message.secondarySigners !== undefined && message.secondarySigners.length !== 0) {
      for (const v of message.secondarySigners) {
        AccountSignature.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MultiAgentSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMultiAgentSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sender = AccountSignature.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.secondarySignerAddresses!.push(reader.string());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.secondarySigners!.push(AccountSignature.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MultiAgentSignature, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MultiAgentSignature | MultiAgentSignature[]>
      | Iterable<MultiAgentSignature | MultiAgentSignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiAgentSignature.encode(p).finish()];
        }
      } else {
        yield* [MultiAgentSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MultiAgentSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MultiAgentSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiAgentSignature.decode(p)];
        }
      } else {
        yield* [MultiAgentSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MultiAgentSignature {
    return {
      sender: isSet(object.sender) ? AccountSignature.fromJSON(object.sender) : undefined,
      secondarySignerAddresses: globalThis.Array.isArray(object?.secondarySignerAddresses)
        ? object.secondarySignerAddresses.map((e: any) => globalThis.String(e))
        : [],
      secondarySigners: globalThis.Array.isArray(object?.secondarySigners)
        ? object.secondarySigners.map((e: any) => AccountSignature.fromJSON(e))
        : [],
    };
  },

  toJSON(message: MultiAgentSignature): unknown {
    const obj: any = {};
    if (message.sender !== undefined) {
      obj.sender = AccountSignature.toJSON(message.sender);
    }
    if (message.secondarySignerAddresses?.length) {
      obj.secondarySignerAddresses = message.secondarySignerAddresses;
    }
    if (message.secondarySigners?.length) {
      obj.secondarySigners = message.secondarySigners.map((e) => AccountSignature.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<MultiAgentSignature>): MultiAgentSignature {
    return MultiAgentSignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MultiAgentSignature>): MultiAgentSignature {
    const message = createBaseMultiAgentSignature();
    message.sender = (object.sender !== undefined && object.sender !== null)
      ? AccountSignature.fromPartial(object.sender)
      : undefined;
    message.secondarySignerAddresses = object.secondarySignerAddresses?.map((e) => e) || [];
    message.secondarySigners = object.secondarySigners?.map((e) => AccountSignature.fromPartial(e)) || [];
    return message;
  },
};

function createBaseFeePayerSignature(): FeePayerSignature {
  return {
    sender: undefined,
    secondarySignerAddresses: [],
    secondarySigners: [],
    feePayerAddress: "",
    feePayerSigner: undefined,
  };
}

export const FeePayerSignature = {
  encode(message: FeePayerSignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sender !== undefined) {
      AccountSignature.encode(message.sender, writer.uint32(10).fork()).ldelim();
    }
    if (message.secondarySignerAddresses !== undefined && message.secondarySignerAddresses.length !== 0) {
      for (const v of message.secondarySignerAddresses) {
        writer.uint32(18).string(v!);
      }
    }
    if (message.secondarySigners !== undefined && message.secondarySigners.length !== 0) {
      for (const v of message.secondarySigners) {
        AccountSignature.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    if (message.feePayerAddress !== undefined && message.feePayerAddress !== "") {
      writer.uint32(34).string(message.feePayerAddress);
    }
    if (message.feePayerSigner !== undefined) {
      AccountSignature.encode(message.feePayerSigner, writer.uint32(42).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): FeePayerSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseFeePayerSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sender = AccountSignature.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.secondarySignerAddresses!.push(reader.string());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.secondarySigners!.push(AccountSignature.decode(reader, reader.uint32()));
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.feePayerAddress = reader.string();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.feePayerSigner = AccountSignature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<FeePayerSignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<FeePayerSignature | FeePayerSignature[]> | Iterable<FeePayerSignature | FeePayerSignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [FeePayerSignature.encode(p).finish()];
        }
      } else {
        yield* [FeePayerSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, FeePayerSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<FeePayerSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [FeePayerSignature.decode(p)];
        }
      } else {
        yield* [FeePayerSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): FeePayerSignature {
    return {
      sender: isSet(object.sender) ? AccountSignature.fromJSON(object.sender) : undefined,
      secondarySignerAddresses: globalThis.Array.isArray(object?.secondarySignerAddresses)
        ? object.secondarySignerAddresses.map((e: any) => globalThis.String(e))
        : [],
      secondarySigners: globalThis.Array.isArray(object?.secondarySigners)
        ? object.secondarySigners.map((e: any) => AccountSignature.fromJSON(e))
        : [],
      feePayerAddress: isSet(object.feePayerAddress) ? globalThis.String(object.feePayerAddress) : "",
      feePayerSigner: isSet(object.feePayerSigner) ? AccountSignature.fromJSON(object.feePayerSigner) : undefined,
    };
  },

  toJSON(message: FeePayerSignature): unknown {
    const obj: any = {};
    if (message.sender !== undefined) {
      obj.sender = AccountSignature.toJSON(message.sender);
    }
    if (message.secondarySignerAddresses?.length) {
      obj.secondarySignerAddresses = message.secondarySignerAddresses;
    }
    if (message.secondarySigners?.length) {
      obj.secondarySigners = message.secondarySigners.map((e) => AccountSignature.toJSON(e));
    }
    if (message.feePayerAddress !== undefined && message.feePayerAddress !== "") {
      obj.feePayerAddress = message.feePayerAddress;
    }
    if (message.feePayerSigner !== undefined) {
      obj.feePayerSigner = AccountSignature.toJSON(message.feePayerSigner);
    }
    return obj;
  },

  create(base?: DeepPartial<FeePayerSignature>): FeePayerSignature {
    return FeePayerSignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<FeePayerSignature>): FeePayerSignature {
    const message = createBaseFeePayerSignature();
    message.sender = (object.sender !== undefined && object.sender !== null)
      ? AccountSignature.fromPartial(object.sender)
      : undefined;
    message.secondarySignerAddresses = object.secondarySignerAddresses?.map((e) => e) || [];
    message.secondarySigners = object.secondarySigners?.map((e) => AccountSignature.fromPartial(e)) || [];
    message.feePayerAddress = object.feePayerAddress ?? "";
    message.feePayerSigner = (object.feePayerSigner !== undefined && object.feePayerSigner !== null)
      ? AccountSignature.fromPartial(object.feePayerSigner)
      : undefined;
    return message;
  },
};

function createBaseAnyPublicKey(): AnyPublicKey {
  return { type: 0, publicKey: new Uint8Array(0) };
}

export const AnyPublicKey = {
  encode(message: AnyPublicKey, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.publicKey !== undefined && message.publicKey.length !== 0) {
      writer.uint32(18).bytes(message.publicKey);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): AnyPublicKey {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseAnyPublicKey();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.publicKey = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<AnyPublicKey, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<AnyPublicKey | AnyPublicKey[]> | Iterable<AnyPublicKey | AnyPublicKey[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AnyPublicKey.encode(p).finish()];
        }
      } else {
        yield* [AnyPublicKey.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, AnyPublicKey>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<AnyPublicKey> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AnyPublicKey.decode(p)];
        }
      } else {
        yield* [AnyPublicKey.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): AnyPublicKey {
    return {
      type: isSet(object.type) ? anyPublicKey_TypeFromJSON(object.type) : 0,
      publicKey: isSet(object.publicKey) ? bytesFromBase64(object.publicKey) : new Uint8Array(0),
    };
  },

  toJSON(message: AnyPublicKey): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = anyPublicKey_TypeToJSON(message.type);
    }
    if (message.publicKey !== undefined && message.publicKey.length !== 0) {
      obj.publicKey = base64FromBytes(message.publicKey);
    }
    return obj;
  },

  create(base?: DeepPartial<AnyPublicKey>): AnyPublicKey {
    return AnyPublicKey.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<AnyPublicKey>): AnyPublicKey {
    const message = createBaseAnyPublicKey();
    message.type = object.type ?? 0;
    message.publicKey = object.publicKey ?? new Uint8Array(0);
    return message;
  },
};

function createBaseAnySignature(): AnySignature {
  return {
    type: 0,
    signature: new Uint8Array(0),
    ed25519: undefined,
    secp256k1Ecdsa: undefined,
    webauthn: undefined,
    keyless: undefined,
  };
}

export const AnySignature = {
  encode(message: AnySignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(18).bytes(message.signature);
    }
    if (message.ed25519 !== undefined) {
      Ed25519.encode(message.ed25519, writer.uint32(26).fork()).ldelim();
    }
    if (message.secp256k1Ecdsa !== undefined) {
      Secp256k1Ecdsa.encode(message.secp256k1Ecdsa, writer.uint32(34).fork()).ldelim();
    }
    if (message.webauthn !== undefined) {
      WebAuthn.encode(message.webauthn, writer.uint32(42).fork()).ldelim();
    }
    if (message.keyless !== undefined) {
      Keyless.encode(message.keyless, writer.uint32(50).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): AnySignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseAnySignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signature = reader.bytes();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ed25519 = Ed25519.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.secp256k1Ecdsa = Secp256k1Ecdsa.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.webauthn = WebAuthn.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.keyless = Keyless.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<AnySignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<AnySignature | AnySignature[]> | Iterable<AnySignature | AnySignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AnySignature.encode(p).finish()];
        }
      } else {
        yield* [AnySignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, AnySignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<AnySignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AnySignature.decode(p)];
        }
      } else {
        yield* [AnySignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): AnySignature {
    return {
      type: isSet(object.type) ? anySignature_TypeFromJSON(object.type) : 0,
      signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0),
      ed25519: isSet(object.ed25519) ? Ed25519.fromJSON(object.ed25519) : undefined,
      secp256k1Ecdsa: isSet(object.secp256k1Ecdsa) ? Secp256k1Ecdsa.fromJSON(object.secp256k1Ecdsa) : undefined,
      webauthn: isSet(object.webauthn) ? WebAuthn.fromJSON(object.webauthn) : undefined,
      keyless: isSet(object.keyless) ? Keyless.fromJSON(object.keyless) : undefined,
    };
  },

  toJSON(message: AnySignature): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = anySignature_TypeToJSON(message.type);
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    if (message.ed25519 !== undefined) {
      obj.ed25519 = Ed25519.toJSON(message.ed25519);
    }
    if (message.secp256k1Ecdsa !== undefined) {
      obj.secp256k1Ecdsa = Secp256k1Ecdsa.toJSON(message.secp256k1Ecdsa);
    }
    if (message.webauthn !== undefined) {
      obj.webauthn = WebAuthn.toJSON(message.webauthn);
    }
    if (message.keyless !== undefined) {
      obj.keyless = Keyless.toJSON(message.keyless);
    }
    return obj;
  },

  create(base?: DeepPartial<AnySignature>): AnySignature {
    return AnySignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<AnySignature>): AnySignature {
    const message = createBaseAnySignature();
    message.type = object.type ?? 0;
    message.signature = object.signature ?? new Uint8Array(0);
    message.ed25519 = (object.ed25519 !== undefined && object.ed25519 !== null)
      ? Ed25519.fromPartial(object.ed25519)
      : undefined;
    message.secp256k1Ecdsa = (object.secp256k1Ecdsa !== undefined && object.secp256k1Ecdsa !== null)
      ? Secp256k1Ecdsa.fromPartial(object.secp256k1Ecdsa)
      : undefined;
    message.webauthn = (object.webauthn !== undefined && object.webauthn !== null)
      ? WebAuthn.fromPartial(object.webauthn)
      : undefined;
    message.keyless = (object.keyless !== undefined && object.keyless !== null)
      ? Keyless.fromPartial(object.keyless)
      : undefined;
    return message;
  },
};

function createBaseEd25519(): Ed25519 {
  return { signature: new Uint8Array(0) };
}

export const Ed25519 = {
  encode(message: Ed25519, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(10).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Ed25519 {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEd25519();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Ed25519, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Ed25519 | Ed25519[]> | Iterable<Ed25519 | Ed25519[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Ed25519.encode(p).finish()];
        }
      } else {
        yield* [Ed25519.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Ed25519>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Ed25519> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Ed25519.decode(p)];
        }
      } else {
        yield* [Ed25519.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Ed25519 {
    return { signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0) };
  },

  toJSON(message: Ed25519): unknown {
    const obj: any = {};
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<Ed25519>): Ed25519 {
    return Ed25519.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Ed25519>): Ed25519 {
    const message = createBaseEd25519();
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseSecp256k1Ecdsa(): Secp256k1Ecdsa {
  return { signature: new Uint8Array(0) };
}

export const Secp256k1Ecdsa = {
  encode(message: Secp256k1Ecdsa, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(10).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Secp256k1Ecdsa {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSecp256k1Ecdsa();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Secp256k1Ecdsa, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Secp256k1Ecdsa | Secp256k1Ecdsa[]> | Iterable<Secp256k1Ecdsa | Secp256k1Ecdsa[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Secp256k1Ecdsa.encode(p).finish()];
        }
      } else {
        yield* [Secp256k1Ecdsa.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Secp256k1Ecdsa>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Secp256k1Ecdsa> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Secp256k1Ecdsa.decode(p)];
        }
      } else {
        yield* [Secp256k1Ecdsa.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Secp256k1Ecdsa {
    return { signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0) };
  },

  toJSON(message: Secp256k1Ecdsa): unknown {
    const obj: any = {};
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<Secp256k1Ecdsa>): Secp256k1Ecdsa {
    return Secp256k1Ecdsa.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Secp256k1Ecdsa>): Secp256k1Ecdsa {
    const message = createBaseSecp256k1Ecdsa();
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseWebAuthn(): WebAuthn {
  return { signature: new Uint8Array(0) };
}

export const WebAuthn = {
  encode(message: WebAuthn, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(10).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WebAuthn {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWebAuthn();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WebAuthn, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WebAuthn | WebAuthn[]> | Iterable<WebAuthn | WebAuthn[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WebAuthn.encode(p).finish()];
        }
      } else {
        yield* [WebAuthn.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WebAuthn>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WebAuthn> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WebAuthn.decode(p)];
        }
      } else {
        yield* [WebAuthn.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WebAuthn {
    return { signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0) };
  },

  toJSON(message: WebAuthn): unknown {
    const obj: any = {};
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<WebAuthn>): WebAuthn {
    return WebAuthn.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WebAuthn>): WebAuthn {
    const message = createBaseWebAuthn();
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseKeyless(): Keyless {
  return { signature: new Uint8Array(0) };
}

export const Keyless = {
  encode(message: Keyless, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(10).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Keyless {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseKeyless();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Keyless, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Keyless | Keyless[]> | Iterable<Keyless | Keyless[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Keyless.encode(p).finish()];
        }
      } else {
        yield* [Keyless.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Keyless>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Keyless> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Keyless.decode(p)];
        }
      } else {
        yield* [Keyless.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Keyless {
    return { signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0) };
  },

  toJSON(message: Keyless): unknown {
    const obj: any = {};
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<Keyless>): Keyless {
    return Keyless.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Keyless>): Keyless {
    const message = createBaseKeyless();
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseSingleKeySignature(): SingleKeySignature {
  return { publicKey: undefined, signature: undefined };
}

export const SingleKeySignature = {
  encode(message: SingleKeySignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.publicKey !== undefined) {
      AnyPublicKey.encode(message.publicKey, writer.uint32(10).fork()).ldelim();
    }
    if (message.signature !== undefined) {
      AnySignature.encode(message.signature, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SingleKeySignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSingleKeySignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.publicKey = AnyPublicKey.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signature = AnySignature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<SingleKeySignature, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<SingleKeySignature | SingleKeySignature[]>
      | Iterable<SingleKeySignature | SingleKeySignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SingleKeySignature.encode(p).finish()];
        }
      } else {
        yield* [SingleKeySignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, SingleKeySignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<SingleKeySignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SingleKeySignature.decode(p)];
        }
      } else {
        yield* [SingleKeySignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): SingleKeySignature {
    return {
      publicKey: isSet(object.publicKey) ? AnyPublicKey.fromJSON(object.publicKey) : undefined,
      signature: isSet(object.signature) ? AnySignature.fromJSON(object.signature) : undefined,
    };
  },

  toJSON(message: SingleKeySignature): unknown {
    const obj: any = {};
    if (message.publicKey !== undefined) {
      obj.publicKey = AnyPublicKey.toJSON(message.publicKey);
    }
    if (message.signature !== undefined) {
      obj.signature = AnySignature.toJSON(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<SingleKeySignature>): SingleKeySignature {
    return SingleKeySignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<SingleKeySignature>): SingleKeySignature {
    const message = createBaseSingleKeySignature();
    message.publicKey = (object.publicKey !== undefined && object.publicKey !== null)
      ? AnyPublicKey.fromPartial(object.publicKey)
      : undefined;
    message.signature = (object.signature !== undefined && object.signature !== null)
      ? AnySignature.fromPartial(object.signature)
      : undefined;
    return message;
  },
};

function createBaseIndexedSignature(): IndexedSignature {
  return { index: 0, signature: undefined };
}

export const IndexedSignature = {
  encode(message: IndexedSignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.index !== undefined && message.index !== 0) {
      writer.uint32(8).uint32(message.index);
    }
    if (message.signature !== undefined) {
      AnySignature.encode(message.signature, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): IndexedSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseIndexedSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.index = reader.uint32();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signature = AnySignature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<IndexedSignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<IndexedSignature | IndexedSignature[]> | Iterable<IndexedSignature | IndexedSignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [IndexedSignature.encode(p).finish()];
        }
      } else {
        yield* [IndexedSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, IndexedSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<IndexedSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [IndexedSignature.decode(p)];
        }
      } else {
        yield* [IndexedSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): IndexedSignature {
    return {
      index: isSet(object.index) ? globalThis.Number(object.index) : 0,
      signature: isSet(object.signature) ? AnySignature.fromJSON(object.signature) : undefined,
    };
  },

  toJSON(message: IndexedSignature): unknown {
    const obj: any = {};
    if (message.index !== undefined && message.index !== 0) {
      obj.index = Math.round(message.index);
    }
    if (message.signature !== undefined) {
      obj.signature = AnySignature.toJSON(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<IndexedSignature>): IndexedSignature {
    return IndexedSignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<IndexedSignature>): IndexedSignature {
    const message = createBaseIndexedSignature();
    message.index = object.index ?? 0;
    message.signature = (object.signature !== undefined && object.signature !== null)
      ? AnySignature.fromPartial(object.signature)
      : undefined;
    return message;
  },
};

function createBaseMultiKeySignature(): MultiKeySignature {
  return { publicKeys: [], signatures: [], signaturesRequired: 0 };
}

export const MultiKeySignature = {
  encode(message: MultiKeySignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.publicKeys !== undefined && message.publicKeys.length !== 0) {
      for (const v of message.publicKeys) {
        AnyPublicKey.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.signatures !== undefined && message.signatures.length !== 0) {
      for (const v of message.signatures) {
        IndexedSignature.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    if (message.signaturesRequired !== undefined && message.signaturesRequired !== 0) {
      writer.uint32(24).uint32(message.signaturesRequired);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MultiKeySignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMultiKeySignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.publicKeys!.push(AnyPublicKey.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signatures!.push(IndexedSignature.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.signaturesRequired = reader.uint32();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<MultiKeySignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<MultiKeySignature | MultiKeySignature[]> | Iterable<MultiKeySignature | MultiKeySignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiKeySignature.encode(p).finish()];
        }
      } else {
        yield* [MultiKeySignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MultiKeySignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MultiKeySignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MultiKeySignature.decode(p)];
        }
      } else {
        yield* [MultiKeySignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MultiKeySignature {
    return {
      publicKeys: globalThis.Array.isArray(object?.publicKeys)
        ? object.publicKeys.map((e: any) => AnyPublicKey.fromJSON(e))
        : [],
      signatures: globalThis.Array.isArray(object?.signatures)
        ? object.signatures.map((e: any) => IndexedSignature.fromJSON(e))
        : [],
      signaturesRequired: isSet(object.signaturesRequired) ? globalThis.Number(object.signaturesRequired) : 0,
    };
  },

  toJSON(message: MultiKeySignature): unknown {
    const obj: any = {};
    if (message.publicKeys?.length) {
      obj.publicKeys = message.publicKeys.map((e) => AnyPublicKey.toJSON(e));
    }
    if (message.signatures?.length) {
      obj.signatures = message.signatures.map((e) => IndexedSignature.toJSON(e));
    }
    if (message.signaturesRequired !== undefined && message.signaturesRequired !== 0) {
      obj.signaturesRequired = Math.round(message.signaturesRequired);
    }
    return obj;
  },

  create(base?: DeepPartial<MultiKeySignature>): MultiKeySignature {
    return MultiKeySignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MultiKeySignature>): MultiKeySignature {
    const message = createBaseMultiKeySignature();
    message.publicKeys = object.publicKeys?.map((e) => AnyPublicKey.fromPartial(e)) || [];
    message.signatures = object.signatures?.map((e) => IndexedSignature.fromPartial(e)) || [];
    message.signaturesRequired = object.signaturesRequired ?? 0;
    return message;
  },
};

function createBaseAbstractSignature(): AbstractSignature {
  return { functionInfo: "", signature: new Uint8Array(0) };
}

export const AbstractSignature = {
  encode(message: AbstractSignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.functionInfo !== undefined && message.functionInfo !== "") {
      writer.uint32(10).string(message.functionInfo);
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      writer.uint32(18).bytes(message.signature);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): AbstractSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseAbstractSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.functionInfo = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.signature = reader.bytes();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<AbstractSignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<AbstractSignature | AbstractSignature[]> | Iterable<AbstractSignature | AbstractSignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AbstractSignature.encode(p).finish()];
        }
      } else {
        yield* [AbstractSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, AbstractSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<AbstractSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AbstractSignature.decode(p)];
        }
      } else {
        yield* [AbstractSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): AbstractSignature {
    return {
      functionInfo: isSet(object.functionInfo) ? globalThis.String(object.functionInfo) : "",
      signature: isSet(object.signature) ? bytesFromBase64(object.signature) : new Uint8Array(0),
    };
  },

  toJSON(message: AbstractSignature): unknown {
    const obj: any = {};
    if (message.functionInfo !== undefined && message.functionInfo !== "") {
      obj.functionInfo = message.functionInfo;
    }
    if (message.signature !== undefined && message.signature.length !== 0) {
      obj.signature = base64FromBytes(message.signature);
    }
    return obj;
  },

  create(base?: DeepPartial<AbstractSignature>): AbstractSignature {
    return AbstractSignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<AbstractSignature>): AbstractSignature {
    const message = createBaseAbstractSignature();
    message.functionInfo = object.functionInfo ?? "";
    message.signature = object.signature ?? new Uint8Array(0);
    return message;
  },
};

function createBaseSingleSender(): SingleSender {
  return { sender: undefined };
}

export const SingleSender = {
  encode(message: SingleSender, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sender !== undefined) {
      AccountSignature.encode(message.sender, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SingleSender {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSingleSender();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sender = AccountSignature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<SingleSender, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<SingleSender | SingleSender[]> | Iterable<SingleSender | SingleSender[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SingleSender.encode(p).finish()];
        }
      } else {
        yield* [SingleSender.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, SingleSender>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<SingleSender> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SingleSender.decode(p)];
        }
      } else {
        yield* [SingleSender.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): SingleSender {
    return { sender: isSet(object.sender) ? AccountSignature.fromJSON(object.sender) : undefined };
  },

  toJSON(message: SingleSender): unknown {
    const obj: any = {};
    if (message.sender !== undefined) {
      obj.sender = AccountSignature.toJSON(message.sender);
    }
    return obj;
  },

  create(base?: DeepPartial<SingleSender>): SingleSender {
    return SingleSender.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<SingleSender>): SingleSender {
    const message = createBaseSingleSender();
    message.sender = (object.sender !== undefined && object.sender !== null)
      ? AccountSignature.fromPartial(object.sender)
      : undefined;
    return message;
  },
};

function createBaseAccountSignature(): AccountSignature {
  return {
    type: 0,
    ed25519: undefined,
    multiEd25519: undefined,
    singleKeySignature: undefined,
    multiKeySignature: undefined,
    abstraction: undefined,
  };
}

export const AccountSignature = {
  encode(message: AccountSignature, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.ed25519 !== undefined) {
      Ed25519Signature.encode(message.ed25519, writer.uint32(18).fork()).ldelim();
    }
    if (message.multiEd25519 !== undefined) {
      MultiEd25519Signature.encode(message.multiEd25519, writer.uint32(26).fork()).ldelim();
    }
    if (message.singleKeySignature !== undefined) {
      SingleKeySignature.encode(message.singleKeySignature, writer.uint32(42).fork()).ldelim();
    }
    if (message.multiKeySignature !== undefined) {
      MultiKeySignature.encode(message.multiKeySignature, writer.uint32(50).fork()).ldelim();
    }
    if (message.abstraction !== undefined) {
      AbstractSignature.encode(message.abstraction, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): AccountSignature {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseAccountSignature();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ed25519 = Ed25519Signature.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.multiEd25519 = MultiEd25519Signature.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.singleKeySignature = SingleKeySignature.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.multiKeySignature = MultiKeySignature.decode(reader, reader.uint32());
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.abstraction = AbstractSignature.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<AccountSignature, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<AccountSignature | AccountSignature[]> | Iterable<AccountSignature | AccountSignature[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AccountSignature.encode(p).finish()];
        }
      } else {
        yield* [AccountSignature.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, AccountSignature>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<AccountSignature> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [AccountSignature.decode(p)];
        }
      } else {
        yield* [AccountSignature.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): AccountSignature {
    return {
      type: isSet(object.type) ? accountSignature_TypeFromJSON(object.type) : 0,
      ed25519: isSet(object.ed25519) ? Ed25519Signature.fromJSON(object.ed25519) : undefined,
      multiEd25519: isSet(object.multiEd25519) ? MultiEd25519Signature.fromJSON(object.multiEd25519) : undefined,
      singleKeySignature: isSet(object.singleKeySignature)
        ? SingleKeySignature.fromJSON(object.singleKeySignature)
        : undefined,
      multiKeySignature: isSet(object.multiKeySignature)
        ? MultiKeySignature.fromJSON(object.multiKeySignature)
        : undefined,
      abstraction: isSet(object.abstraction) ? AbstractSignature.fromJSON(object.abstraction) : undefined,
    };
  },

  toJSON(message: AccountSignature): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = accountSignature_TypeToJSON(message.type);
    }
    if (message.ed25519 !== undefined) {
      obj.ed25519 = Ed25519Signature.toJSON(message.ed25519);
    }
    if (message.multiEd25519 !== undefined) {
      obj.multiEd25519 = MultiEd25519Signature.toJSON(message.multiEd25519);
    }
    if (message.singleKeySignature !== undefined) {
      obj.singleKeySignature = SingleKeySignature.toJSON(message.singleKeySignature);
    }
    if (message.multiKeySignature !== undefined) {
      obj.multiKeySignature = MultiKeySignature.toJSON(message.multiKeySignature);
    }
    if (message.abstraction !== undefined) {
      obj.abstraction = AbstractSignature.toJSON(message.abstraction);
    }
    return obj;
  },

  create(base?: DeepPartial<AccountSignature>): AccountSignature {
    return AccountSignature.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<AccountSignature>): AccountSignature {
    const message = createBaseAccountSignature();
    message.type = object.type ?? 0;
    message.ed25519 = (object.ed25519 !== undefined && object.ed25519 !== null)
      ? Ed25519Signature.fromPartial(object.ed25519)
      : undefined;
    message.multiEd25519 = (object.multiEd25519 !== undefined && object.multiEd25519 !== null)
      ? MultiEd25519Signature.fromPartial(object.multiEd25519)
      : undefined;
    message.singleKeySignature = (object.singleKeySignature !== undefined && object.singleKeySignature !== null)
      ? SingleKeySignature.fromPartial(object.singleKeySignature)
      : undefined;
    message.multiKeySignature = (object.multiKeySignature !== undefined && object.multiKeySignature !== null)
      ? MultiKeySignature.fromPartial(object.multiKeySignature)
      : undefined;
    message.abstraction = (object.abstraction !== undefined && object.abstraction !== null)
      ? AbstractSignature.fromPartial(object.abstraction)
      : undefined;
    return message;
  },
};

function createBaseTransactionSizeInfo(): TransactionSizeInfo {
  return { transactionBytes: 0, eventSizeInfo: [], writeOpSizeInfo: [] };
}

export const TransactionSizeInfo = {
  encode(message: TransactionSizeInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactionBytes !== undefined && message.transactionBytes !== 0) {
      writer.uint32(8).uint32(message.transactionBytes);
    }
    if (message.eventSizeInfo !== undefined && message.eventSizeInfo.length !== 0) {
      for (const v of message.eventSizeInfo) {
        EventSizeInfo.encode(v!, writer.uint32(18).fork()).ldelim();
      }
    }
    if (message.writeOpSizeInfo !== undefined && message.writeOpSizeInfo.length !== 0) {
      for (const v of message.writeOpSizeInfo) {
        WriteOpSizeInfo.encode(v!, writer.uint32(26).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionSizeInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionSizeInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.transactionBytes = reader.uint32();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.eventSizeInfo!.push(EventSizeInfo.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.writeOpSizeInfo!.push(WriteOpSizeInfo.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<TransactionSizeInfo, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionSizeInfo | TransactionSizeInfo[]>
      | Iterable<TransactionSizeInfo | TransactionSizeInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionSizeInfo.encode(p).finish()];
        }
      } else {
        yield* [TransactionSizeInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionSizeInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionSizeInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionSizeInfo.decode(p)];
        }
      } else {
        yield* [TransactionSizeInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionSizeInfo {
    return {
      transactionBytes: isSet(object.transactionBytes) ? globalThis.Number(object.transactionBytes) : 0,
      eventSizeInfo: globalThis.Array.isArray(object?.eventSizeInfo)
        ? object.eventSizeInfo.map((e: any) => EventSizeInfo.fromJSON(e))
        : [],
      writeOpSizeInfo: globalThis.Array.isArray(object?.writeOpSizeInfo)
        ? object.writeOpSizeInfo.map((e: any) => WriteOpSizeInfo.fromJSON(e))
        : [],
    };
  },

  toJSON(message: TransactionSizeInfo): unknown {
    const obj: any = {};
    if (message.transactionBytes !== undefined && message.transactionBytes !== 0) {
      obj.transactionBytes = Math.round(message.transactionBytes);
    }
    if (message.eventSizeInfo?.length) {
      obj.eventSizeInfo = message.eventSizeInfo.map((e) => EventSizeInfo.toJSON(e));
    }
    if (message.writeOpSizeInfo?.length) {
      obj.writeOpSizeInfo = message.writeOpSizeInfo.map((e) => WriteOpSizeInfo.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionSizeInfo>): TransactionSizeInfo {
    return TransactionSizeInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionSizeInfo>): TransactionSizeInfo {
    const message = createBaseTransactionSizeInfo();
    message.transactionBytes = object.transactionBytes ?? 0;
    message.eventSizeInfo = object.eventSizeInfo?.map((e) => EventSizeInfo.fromPartial(e)) || [];
    message.writeOpSizeInfo = object.writeOpSizeInfo?.map((e) => WriteOpSizeInfo.fromPartial(e)) || [];
    return message;
  },
};

function createBaseEventSizeInfo(): EventSizeInfo {
  return { typeTagBytes: 0, totalBytes: 0 };
}

export const EventSizeInfo = {
  encode(message: EventSizeInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.typeTagBytes !== undefined && message.typeTagBytes !== 0) {
      writer.uint32(8).uint32(message.typeTagBytes);
    }
    if (message.totalBytes !== undefined && message.totalBytes !== 0) {
      writer.uint32(16).uint32(message.totalBytes);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EventSizeInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventSizeInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.typeTagBytes = reader.uint32();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.totalBytes = reader.uint32();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<EventSizeInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventSizeInfo | EventSizeInfo[]> | Iterable<EventSizeInfo | EventSizeInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventSizeInfo.encode(p).finish()];
        }
      } else {
        yield* [EventSizeInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventSizeInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventSizeInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventSizeInfo.decode(p)];
        }
      } else {
        yield* [EventSizeInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventSizeInfo {
    return {
      typeTagBytes: isSet(object.typeTagBytes) ? globalThis.Number(object.typeTagBytes) : 0,
      totalBytes: isSet(object.totalBytes) ? globalThis.Number(object.totalBytes) : 0,
    };
  },

  toJSON(message: EventSizeInfo): unknown {
    const obj: any = {};
    if (message.typeTagBytes !== undefined && message.typeTagBytes !== 0) {
      obj.typeTagBytes = Math.round(message.typeTagBytes);
    }
    if (message.totalBytes !== undefined && message.totalBytes !== 0) {
      obj.totalBytes = Math.round(message.totalBytes);
    }
    return obj;
  },

  create(base?: DeepPartial<EventSizeInfo>): EventSizeInfo {
    return EventSizeInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventSizeInfo>): EventSizeInfo {
    const message = createBaseEventSizeInfo();
    message.typeTagBytes = object.typeTagBytes ?? 0;
    message.totalBytes = object.totalBytes ?? 0;
    return message;
  },
};

function createBaseWriteOpSizeInfo(): WriteOpSizeInfo {
  return { keyBytes: 0, valueBytes: 0 };
}

export const WriteOpSizeInfo = {
  encode(message: WriteOpSizeInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.keyBytes !== undefined && message.keyBytes !== 0) {
      writer.uint32(8).uint32(message.keyBytes);
    }
    if (message.valueBytes !== undefined && message.valueBytes !== 0) {
      writer.uint32(16).uint32(message.valueBytes);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WriteOpSizeInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWriteOpSizeInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.keyBytes = reader.uint32();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.valueBytes = reader.uint32();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<WriteOpSizeInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<WriteOpSizeInfo | WriteOpSizeInfo[]> | Iterable<WriteOpSizeInfo | WriteOpSizeInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteOpSizeInfo.encode(p).finish()];
        }
      } else {
        yield* [WriteOpSizeInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, WriteOpSizeInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<WriteOpSizeInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [WriteOpSizeInfo.decode(p)];
        }
      } else {
        yield* [WriteOpSizeInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): WriteOpSizeInfo {
    return {
      keyBytes: isSet(object.keyBytes) ? globalThis.Number(object.keyBytes) : 0,
      valueBytes: isSet(object.valueBytes) ? globalThis.Number(object.valueBytes) : 0,
    };
  },

  toJSON(message: WriteOpSizeInfo): unknown {
    const obj: any = {};
    if (message.keyBytes !== undefined && message.keyBytes !== 0) {
      obj.keyBytes = Math.round(message.keyBytes);
    }
    if (message.valueBytes !== undefined && message.valueBytes !== 0) {
      obj.valueBytes = Math.round(message.valueBytes);
    }
    return obj;
  },

  create(base?: DeepPartial<WriteOpSizeInfo>): WriteOpSizeInfo {
    return WriteOpSizeInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<WriteOpSizeInfo>): WriteOpSizeInfo {
    const message = createBaseWriteOpSizeInfo();
    message.keyBytes = object.keyBytes ?? 0;
    message.valueBytes = object.valueBytes ?? 0;
    return message;
  },
};

function bytesFromBase64(b64: string): Uint8Array {
  if ((globalThis as any).Buffer) {
    return Uint8Array.from(globalThis.Buffer.from(b64, "base64"));
  } else {
    const bin = globalThis.atob(b64);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; ++i) {
      arr[i] = bin.charCodeAt(i);
    }
    return arr;
  }
}

function base64FromBytes(arr: Uint8Array): string {
  if ((globalThis as any).Buffer) {
    return globalThis.Buffer.from(arr).toString("base64");
  } else {
    const bin: string[] = [];
    arr.forEach((byte) => {
      bin.push(globalThis.String.fromCharCode(byte));
    });
    return globalThis.btoa(bin.join(""));
  }
}

type Builtin = Date | Function | Uint8Array | string | number | boolean | bigint | undefined;

type DeepPartial<T> = T extends Builtin ? T
  : T extends globalThis.Array<infer U> ? globalThis.Array<DeepPartial<U>>
  : T extends ReadonlyArray<infer U> ? ReadonlyArray<DeepPartial<U>>
  : T extends {} ? { [K in keyof T]?: DeepPartial<T[K]> }
  : Partial<T>;

function longToBigint(long: Long) {
  return BigInt(long.toString());
}

if (_m0.util.Long !== Long) {
  _m0.util.Long = Long as any;
  _m0.configure();
}

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
