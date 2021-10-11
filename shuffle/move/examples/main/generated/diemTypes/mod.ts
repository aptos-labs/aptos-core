
import { Serializer, Deserializer } from '../serde/mod.ts';
import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';
import { Optional, Seq, Tuple, ListTuple, unit, bool, int8, int16, int32, int64, int128, uint8, uint16, uint32, uint64, uint128, float32, float64, char, str, bytes } from '../serde/mod.ts';

export class AccessPath {

constructor (public address: AccountAddress, public path: bytes) {
}

public serialize(serializer: Serializer): void {
  this.address.serialize(serializer);
  serializer.serializeBytes(this.path);
}

static deserialize(deserializer: Deserializer): AccessPath {
  const address = AccountAddress.deserialize(deserializer);
  const path = deserializer.deserializeBytes();
  return new AccessPath(address,path);
}

}
export class AccountAddress {

constructor (public value: ListTuple<[uint8]>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeArray16U8Array(this.value, serializer);
}

static deserialize(deserializer: Deserializer): AccountAddress {
  const value = Helpers.deserializeArray16U8Array(deserializer);
  return new AccountAddress(value);
}

}
export abstract class AccountAuthenticator {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): AccountAuthenticator {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return AccountAuthenticatorVariantEd25519.load(deserializer);
    case 1: return AccountAuthenticatorVariantMultiEd25519.load(deserializer);
    default: throw new Error("Unknown variant index for AccountAuthenticator: " + index);
  }
}
}


export class AccountAuthenticatorVariantEd25519 extends AccountAuthenticator {

constructor (public public_key: Ed25519PublicKey, public signature: Ed25519Signature) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.public_key.serialize(serializer);
  this.signature.serialize(serializer);
}

static load(deserializer: Deserializer): AccountAuthenticatorVariantEd25519 {
  const public_key = Ed25519PublicKey.deserialize(deserializer);
  const signature = Ed25519Signature.deserialize(deserializer);
  return new AccountAuthenticatorVariantEd25519(public_key,signature);
}

}

export class AccountAuthenticatorVariantMultiEd25519 extends AccountAuthenticator {

constructor (public public_key: MultiEd25519PublicKey, public signature: MultiEd25519Signature) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.public_key.serialize(serializer);
  this.signature.serialize(serializer);
}

static load(deserializer: Deserializer): AccountAuthenticatorVariantMultiEd25519 {
  const public_key = MultiEd25519PublicKey.deserialize(deserializer);
  const signature = MultiEd25519Signature.deserialize(deserializer);
  return new AccountAuthenticatorVariantMultiEd25519(public_key,signature);
}

}
export class BlockMetadata {

constructor (public id: HashValue, public round: uint64, public timestamp_usecs: uint64, public previous_block_votes: Seq<AccountAddress>, public proposer: AccountAddress) {
}

public serialize(serializer: Serializer): void {
  this.id.serialize(serializer);
  serializer.serializeU64(this.round);
  serializer.serializeU64(this.timestamp_usecs);
  Helpers.serializeVectorAccountAddress(this.previous_block_votes, serializer);
  this.proposer.serialize(serializer);
}

static deserialize(deserializer: Deserializer): BlockMetadata {
  const id = HashValue.deserialize(deserializer);
  const round = deserializer.deserializeU64();
  const timestamp_usecs = deserializer.deserializeU64();
  const previous_block_votes = Helpers.deserializeVectorAccountAddress(deserializer);
  const proposer = AccountAddress.deserialize(deserializer);
  return new BlockMetadata(id,round,timestamp_usecs,previous_block_votes,proposer);
}

}
export class ChainId {

constructor (public value: uint8) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeU8(this.value);
}

static deserialize(deserializer: Deserializer): ChainId {
  const value = deserializer.deserializeU8();
  return new ChainId(value);
}

}
export class ChangeSet {

constructor (public write_set: WriteSet, public events: Seq<ContractEvent>) {
}

public serialize(serializer: Serializer): void {
  this.write_set.serialize(serializer);
  Helpers.serializeVectorContractEvent(this.events, serializer);
}

static deserialize(deserializer: Deserializer): ChangeSet {
  const write_set = WriteSet.deserialize(deserializer);
  const events = Helpers.deserializeVectorContractEvent(deserializer);
  return new ChangeSet(write_set,events);
}

}
export abstract class CoinTradeMetadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): CoinTradeMetadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return CoinTradeMetadataVariantCoinTradeMetadataV0.load(deserializer);
    default: throw new Error("Unknown variant index for CoinTradeMetadata: " + index);
  }
}
}


export class CoinTradeMetadataVariantCoinTradeMetadataV0 extends CoinTradeMetadata {

constructor (public value: CoinTradeMetadataV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): CoinTradeMetadataVariantCoinTradeMetadataV0 {
  const value = CoinTradeMetadataV0.deserialize(deserializer);
  return new CoinTradeMetadataVariantCoinTradeMetadataV0(value);
}

}
export class CoinTradeMetadataV0 {

constructor (public trade_ids: Seq<str>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeVectorStr(this.trade_ids, serializer);
}

static deserialize(deserializer: Deserializer): CoinTradeMetadataV0 {
  const trade_ids = Helpers.deserializeVectorStr(deserializer);
  return new CoinTradeMetadataV0(trade_ids);
}

}
export abstract class ContractEvent {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): ContractEvent {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return ContractEventVariantV0.load(deserializer);
    default: throw new Error("Unknown variant index for ContractEvent: " + index);
  }
}
}


export class ContractEventVariantV0 extends ContractEvent {

constructor (public value: ContractEventV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): ContractEventVariantV0 {
  const value = ContractEventV0.deserialize(deserializer);
  return new ContractEventVariantV0(value);
}

}
export class ContractEventV0 {

constructor (public key: EventKey, public sequence_number: uint64, public type_tag: TypeTag, public event_data: bytes) {
}

public serialize(serializer: Serializer): void {
  this.key.serialize(serializer);
  serializer.serializeU64(this.sequence_number);
  this.type_tag.serialize(serializer);
  serializer.serializeBytes(this.event_data);
}

static deserialize(deserializer: Deserializer): ContractEventV0 {
  const key = EventKey.deserialize(deserializer);
  const sequence_number = deserializer.deserializeU64();
  const type_tag = TypeTag.deserialize(deserializer);
  const event_data = deserializer.deserializeBytes();
  return new ContractEventV0(key,sequence_number,type_tag,event_data);
}

}
export class Ed25519PublicKey {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): Ed25519PublicKey {
  const value = deserializer.deserializeBytes();
  return new Ed25519PublicKey(value);
}

}
export class Ed25519Signature {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): Ed25519Signature {
  const value = deserializer.deserializeBytes();
  return new Ed25519Signature(value);
}

}
export class EventKey {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): EventKey {
  const value = deserializer.deserializeBytes();
  return new EventKey(value);
}

}
export abstract class GeneralMetadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): GeneralMetadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return GeneralMetadataVariantGeneralMetadataVersion0.load(deserializer);
    default: throw new Error("Unknown variant index for GeneralMetadata: " + index);
  }
}
}


export class GeneralMetadataVariantGeneralMetadataVersion0 extends GeneralMetadata {

constructor (public value: GeneralMetadataV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): GeneralMetadataVariantGeneralMetadataVersion0 {
  const value = GeneralMetadataV0.deserialize(deserializer);
  return new GeneralMetadataVariantGeneralMetadataVersion0(value);
}

}
export class GeneralMetadataV0 {

constructor (public to_subaddress: Optional<bytes>, public from_subaddress: Optional<bytes>, public referenced_event: Optional<uint64>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeOptionBytes(this.to_subaddress, serializer);
  Helpers.serializeOptionBytes(this.from_subaddress, serializer);
  Helpers.serializeOptionU64(this.referenced_event, serializer);
}

static deserialize(deserializer: Deserializer): GeneralMetadataV0 {
  const to_subaddress = Helpers.deserializeOptionBytes(deserializer);
  const from_subaddress = Helpers.deserializeOptionBytes(deserializer);
  const referenced_event = Helpers.deserializeOptionU64(deserializer);
  return new GeneralMetadataV0(to_subaddress,from_subaddress,referenced_event);
}

}
export class HashValue {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): HashValue {
  const value = deserializer.deserializeBytes();
  return new HashValue(value);
}

}
export class Identifier {

constructor (public value: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.value);
}

static deserialize(deserializer: Deserializer): Identifier {
  const value = deserializer.deserializeStr();
  return new Identifier(value);
}

}
export abstract class Metadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Metadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return MetadataVariantUndefined.load(deserializer);
    case 1: return MetadataVariantGeneralMetadata.load(deserializer);
    case 2: return MetadataVariantTravelRuleMetadata.load(deserializer);
    case 3: return MetadataVariantUnstructuredBytesMetadata.load(deserializer);
    case 4: return MetadataVariantRefundMetadata.load(deserializer);
    case 5: return MetadataVariantCoinTradeMetadata.load(deserializer);
    case 6: return MetadataVariantPaymentMetadata.load(deserializer);
    default: throw new Error("Unknown variant index for Metadata: " + index);
  }
}
}


export class MetadataVariantUndefined extends Metadata {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): MetadataVariantUndefined {
  return new MetadataVariantUndefined();
}

}

export class MetadataVariantGeneralMetadata extends Metadata {

constructor (public value: GeneralMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantGeneralMetadata {
  const value = GeneralMetadata.deserialize(deserializer);
  return new MetadataVariantGeneralMetadata(value);
}

}

export class MetadataVariantTravelRuleMetadata extends Metadata {

constructor (public value: TravelRuleMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantTravelRuleMetadata {
  const value = TravelRuleMetadata.deserialize(deserializer);
  return new MetadataVariantTravelRuleMetadata(value);
}

}

export class MetadataVariantUnstructuredBytesMetadata extends Metadata {

constructor (public value: UnstructuredBytesMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantUnstructuredBytesMetadata {
  const value = UnstructuredBytesMetadata.deserialize(deserializer);
  return new MetadataVariantUnstructuredBytesMetadata(value);
}

}

export class MetadataVariantRefundMetadata extends Metadata {

constructor (public value: RefundMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantRefundMetadata {
  const value = RefundMetadata.deserialize(deserializer);
  return new MetadataVariantRefundMetadata(value);
}

}

export class MetadataVariantCoinTradeMetadata extends Metadata {

constructor (public value: CoinTradeMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(5);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantCoinTradeMetadata {
  const value = CoinTradeMetadata.deserialize(deserializer);
  return new MetadataVariantCoinTradeMetadata(value);
}

}

export class MetadataVariantPaymentMetadata extends Metadata {

constructor (public value: PaymentMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(6);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MetadataVariantPaymentMetadata {
  const value = PaymentMetadata.deserialize(deserializer);
  return new MetadataVariantPaymentMetadata(value);
}

}
export class Module {

constructor (public code: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.code);
}

static deserialize(deserializer: Deserializer): Module {
  const code = deserializer.deserializeBytes();
  return new Module(code);
}

}
export class ModuleId {

constructor (public address: AccountAddress, public name: Identifier) {
}

public serialize(serializer: Serializer): void {
  this.address.serialize(serializer);
  this.name.serialize(serializer);
}

static deserialize(deserializer: Deserializer): ModuleId {
  const address = AccountAddress.deserialize(deserializer);
  const name = Identifier.deserialize(deserializer);
  return new ModuleId(address,name);
}

}
export class MultiEd25519PublicKey {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): MultiEd25519PublicKey {
  const value = deserializer.deserializeBytes();
  return new MultiEd25519PublicKey(value);
}

}
export class MultiEd25519Signature {

constructor (public value: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.value);
}

static deserialize(deserializer: Deserializer): MultiEd25519Signature {
  const value = deserializer.deserializeBytes();
  return new MultiEd25519Signature(value);
}

}
export abstract class PaymentMetadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): PaymentMetadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return PaymentMetadataVariantPaymentMetadataVersion0.load(deserializer);
    default: throw new Error("Unknown variant index for PaymentMetadata: " + index);
  }
}
}


export class PaymentMetadataVariantPaymentMetadataVersion0 extends PaymentMetadata {

constructor (public value: PaymentMetadataV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): PaymentMetadataVariantPaymentMetadataVersion0 {
  const value = PaymentMetadataV0.deserialize(deserializer);
  return new PaymentMetadataVariantPaymentMetadataVersion0(value);
}

}
export class PaymentMetadataV0 {

constructor (public reference_id: ListTuple<[uint8]>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeArray16U8Array(this.reference_id, serializer);
}

static deserialize(deserializer: Deserializer): PaymentMetadataV0 {
  const reference_id = Helpers.deserializeArray16U8Array(deserializer);
  return new PaymentMetadataV0(reference_id);
}

}
export class RawTransaction {

constructor (public sender: AccountAddress, public sequence_number: uint64, public payload: TransactionPayload, public max_gas_amount: uint64, public gas_unit_price: uint64, public gas_currency_code: str, public expiration_timestamp_secs: uint64, public chain_id: ChainId) {
}

public serialize(serializer: Serializer): void {
  this.sender.serialize(serializer);
  serializer.serializeU64(this.sequence_number);
  this.payload.serialize(serializer);
  serializer.serializeU64(this.max_gas_amount);
  serializer.serializeU64(this.gas_unit_price);
  serializer.serializeStr(this.gas_currency_code);
  serializer.serializeU64(this.expiration_timestamp_secs);
  this.chain_id.serialize(serializer);
}

static deserialize(deserializer: Deserializer): RawTransaction {
  const sender = AccountAddress.deserialize(deserializer);
  const sequence_number = deserializer.deserializeU64();
  const payload = TransactionPayload.deserialize(deserializer);
  const max_gas_amount = deserializer.deserializeU64();
  const gas_unit_price = deserializer.deserializeU64();
  const gas_currency_code = deserializer.deserializeStr();
  const expiration_timestamp_secs = deserializer.deserializeU64();
  const chain_id = ChainId.deserialize(deserializer);
  return new RawTransaction(sender,sequence_number,payload,max_gas_amount,gas_unit_price,gas_currency_code,expiration_timestamp_secs,chain_id);
}

}
export abstract class RefundMetadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): RefundMetadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return RefundMetadataVariantRefundMetadataV0.load(deserializer);
    default: throw new Error("Unknown variant index for RefundMetadata: " + index);
  }
}
}


export class RefundMetadataVariantRefundMetadataV0 extends RefundMetadata {

constructor (public value: RefundMetadataV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): RefundMetadataVariantRefundMetadataV0 {
  const value = RefundMetadataV0.deserialize(deserializer);
  return new RefundMetadataVariantRefundMetadataV0(value);
}

}
export class RefundMetadataV0 {

constructor (public transaction_version: uint64, public reason: RefundReason) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeU64(this.transaction_version);
  this.reason.serialize(serializer);
}

static deserialize(deserializer: Deserializer): RefundMetadataV0 {
  const transaction_version = deserializer.deserializeU64();
  const reason = RefundReason.deserialize(deserializer);
  return new RefundMetadataV0(transaction_version,reason);
}

}
export abstract class RefundReason {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): RefundReason {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return RefundReasonVariantOtherReason.load(deserializer);
    case 1: return RefundReasonVariantInvalidSubaddress.load(deserializer);
    case 2: return RefundReasonVariantUserInitiatedPartialRefund.load(deserializer);
    case 3: return RefundReasonVariantUserInitiatedFullRefund.load(deserializer);
    case 4: return RefundReasonVariantInvalidReferenceId.load(deserializer);
    default: throw new Error("Unknown variant index for RefundReason: " + index);
  }
}
}


export class RefundReasonVariantOtherReason extends RefundReason {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): RefundReasonVariantOtherReason {
  return new RefundReasonVariantOtherReason();
}

}

export class RefundReasonVariantInvalidSubaddress extends RefundReason {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): RefundReasonVariantInvalidSubaddress {
  return new RefundReasonVariantInvalidSubaddress();
}

}

export class RefundReasonVariantUserInitiatedPartialRefund extends RefundReason {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): RefundReasonVariantUserInitiatedPartialRefund {
  return new RefundReasonVariantUserInitiatedPartialRefund();
}

}

export class RefundReasonVariantUserInitiatedFullRefund extends RefundReason {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
}

static load(deserializer: Deserializer): RefundReasonVariantUserInitiatedFullRefund {
  return new RefundReasonVariantUserInitiatedFullRefund();
}

}

export class RefundReasonVariantInvalidReferenceId extends RefundReason {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
}

static load(deserializer: Deserializer): RefundReasonVariantInvalidReferenceId {
  return new RefundReasonVariantInvalidReferenceId();
}

}
export class Script {

constructor (public code: bytes, public ty_args: Seq<TypeTag>, public args: Seq<TransactionArgument>) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeBytes(this.code);
  Helpers.serializeVectorTypeTag(this.ty_args, serializer);
  Helpers.serializeVectorTransactionArgument(this.args, serializer);
}

static deserialize(deserializer: Deserializer): Script {
  const code = deserializer.deserializeBytes();
  const ty_args = Helpers.deserializeVectorTypeTag(deserializer);
  const args = Helpers.deserializeVectorTransactionArgument(deserializer);
  return new Script(code,ty_args,args);
}

}
export class ScriptFunction {

constructor (public module_name: ModuleId, public function_name: Identifier, public ty_args: Seq<TypeTag>, public args: Seq<bytes>) {
}

public serialize(serializer: Serializer): void {
  this.module_name.serialize(serializer);
  this.function_name.serialize(serializer);
  Helpers.serializeVectorTypeTag(this.ty_args, serializer);
  Helpers.serializeVectorBytes(this.args, serializer);
}

static deserialize(deserializer: Deserializer): ScriptFunction {
  const module_name = ModuleId.deserialize(deserializer);
  const function_name = Identifier.deserialize(deserializer);
  const ty_args = Helpers.deserializeVectorTypeTag(deserializer);
  const args = Helpers.deserializeVectorBytes(deserializer);
  return new ScriptFunction(module_name,function_name,ty_args,args);
}

}
export class SignedTransaction {

constructor (public raw_txn: RawTransaction, public authenticator: TransactionAuthenticator) {
}

public serialize(serializer: Serializer): void {
  this.raw_txn.serialize(serializer);
  this.authenticator.serialize(serializer);
}

static deserialize(deserializer: Deserializer): SignedTransaction {
  const raw_txn = RawTransaction.deserialize(deserializer);
  const authenticator = TransactionAuthenticator.deserialize(deserializer);
  return new SignedTransaction(raw_txn,authenticator);
}

}
export class StructTag {

constructor (public address: AccountAddress, public module_name: Identifier, public name: Identifier, public type_params: Seq<TypeTag>) {
}

public serialize(serializer: Serializer): void {
  this.address.serialize(serializer);
  this.module_name.serialize(serializer);
  this.name.serialize(serializer);
  Helpers.serializeVectorTypeTag(this.type_params, serializer);
}

static deserialize(deserializer: Deserializer): StructTag {
  const address = AccountAddress.deserialize(deserializer);
  const module_name = Identifier.deserialize(deserializer);
  const name = Identifier.deserialize(deserializer);
  const type_params = Helpers.deserializeVectorTypeTag(deserializer);
  return new StructTag(address,module_name,name,type_params);
}

}
export abstract class Transaction {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Transaction {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TransactionVariantUserTransaction.load(deserializer);
    case 1: return TransactionVariantGenesisTransaction.load(deserializer);
    case 2: return TransactionVariantBlockMetadata.load(deserializer);
    default: throw new Error("Unknown variant index for Transaction: " + index);
  }
}
}


export class TransactionVariantUserTransaction extends Transaction {

constructor (public value: SignedTransaction) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionVariantUserTransaction {
  const value = SignedTransaction.deserialize(deserializer);
  return new TransactionVariantUserTransaction(value);
}

}

export class TransactionVariantGenesisTransaction extends Transaction {

constructor (public value: WriteSetPayload) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionVariantGenesisTransaction {
  const value = WriteSetPayload.deserialize(deserializer);
  return new TransactionVariantGenesisTransaction(value);
}

}

export class TransactionVariantBlockMetadata extends Transaction {

constructor (public value: BlockMetadata) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionVariantBlockMetadata {
  const value = BlockMetadata.deserialize(deserializer);
  return new TransactionVariantBlockMetadata(value);
}

}
export abstract class TransactionArgument {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): TransactionArgument {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TransactionArgumentVariantU8.load(deserializer);
    case 1: return TransactionArgumentVariantU64.load(deserializer);
    case 2: return TransactionArgumentVariantU128.load(deserializer);
    case 3: return TransactionArgumentVariantAddress.load(deserializer);
    case 4: return TransactionArgumentVariantU8Vector.load(deserializer);
    case 5: return TransactionArgumentVariantBool.load(deserializer);
    default: throw new Error("Unknown variant index for TransactionArgument: " + index);
  }
}
}


export class TransactionArgumentVariantU8 extends TransactionArgument {

constructor (public value: uint8) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  serializer.serializeU8(this.value);
}

static load(deserializer: Deserializer): TransactionArgumentVariantU8 {
  const value = deserializer.deserializeU8();
  return new TransactionArgumentVariantU8(value);
}

}

export class TransactionArgumentVariantU64 extends TransactionArgument {

constructor (public value: uint64) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeU64(this.value);
}

static load(deserializer: Deserializer): TransactionArgumentVariantU64 {
  const value = deserializer.deserializeU64();
  return new TransactionArgumentVariantU64(value);
}

}

export class TransactionArgumentVariantU128 extends TransactionArgument {

constructor (public value: uint128) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  serializer.serializeU128(this.value);
}

static load(deserializer: Deserializer): TransactionArgumentVariantU128 {
  const value = deserializer.deserializeU128();
  return new TransactionArgumentVariantU128(value);
}

}

export class TransactionArgumentVariantAddress extends TransactionArgument {

constructor (public value: AccountAddress) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionArgumentVariantAddress {
  const value = AccountAddress.deserialize(deserializer);
  return new TransactionArgumentVariantAddress(value);
}

}

export class TransactionArgumentVariantU8Vector extends TransactionArgument {

constructor (public value: bytes) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
  serializer.serializeBytes(this.value);
}

static load(deserializer: Deserializer): TransactionArgumentVariantU8Vector {
  const value = deserializer.deserializeBytes();
  return new TransactionArgumentVariantU8Vector(value);
}

}

export class TransactionArgumentVariantBool extends TransactionArgument {

constructor (public value: bool) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(5);
  serializer.serializeBool(this.value);
}

static load(deserializer: Deserializer): TransactionArgumentVariantBool {
  const value = deserializer.deserializeBool();
  return new TransactionArgumentVariantBool(value);
}

}
export abstract class TransactionAuthenticator {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): TransactionAuthenticator {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TransactionAuthenticatorVariantEd25519.load(deserializer);
    case 1: return TransactionAuthenticatorVariantMultiEd25519.load(deserializer);
    case 2: return TransactionAuthenticatorVariantMultiAgent.load(deserializer);
    default: throw new Error("Unknown variant index for TransactionAuthenticator: " + index);
  }
}
}


export class TransactionAuthenticatorVariantEd25519 extends TransactionAuthenticator {

constructor (public public_key: Ed25519PublicKey, public signature: Ed25519Signature) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.public_key.serialize(serializer);
  this.signature.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionAuthenticatorVariantEd25519 {
  const public_key = Ed25519PublicKey.deserialize(deserializer);
  const signature = Ed25519Signature.deserialize(deserializer);
  return new TransactionAuthenticatorVariantEd25519(public_key,signature);
}

}

export class TransactionAuthenticatorVariantMultiEd25519 extends TransactionAuthenticator {

constructor (public public_key: MultiEd25519PublicKey, public signature: MultiEd25519Signature) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.public_key.serialize(serializer);
  this.signature.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionAuthenticatorVariantMultiEd25519 {
  const public_key = MultiEd25519PublicKey.deserialize(deserializer);
  const signature = MultiEd25519Signature.deserialize(deserializer);
  return new TransactionAuthenticatorVariantMultiEd25519(public_key,signature);
}

}

export class TransactionAuthenticatorVariantMultiAgent extends TransactionAuthenticator {

constructor (public sender: AccountAuthenticator, public secondary_signer_addresses: Seq<AccountAddress>, public secondary_signers: Seq<AccountAuthenticator>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.sender.serialize(serializer);
  Helpers.serializeVectorAccountAddress(this.secondary_signer_addresses, serializer);
  Helpers.serializeVectorAccountAuthenticator(this.secondary_signers, serializer);
}

static load(deserializer: Deserializer): TransactionAuthenticatorVariantMultiAgent {
  const sender = AccountAuthenticator.deserialize(deserializer);
  const secondary_signer_addresses = Helpers.deserializeVectorAccountAddress(deserializer);
  const secondary_signers = Helpers.deserializeVectorAccountAuthenticator(deserializer);
  return new TransactionAuthenticatorVariantMultiAgent(sender,secondary_signer_addresses,secondary_signers);
}

}
export abstract class TransactionPayload {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): TransactionPayload {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TransactionPayloadVariantWriteSet.load(deserializer);
    case 1: return TransactionPayloadVariantScript.load(deserializer);
    case 2: return TransactionPayloadVariantModule.load(deserializer);
    case 3: return TransactionPayloadVariantScriptFunction.load(deserializer);
    default: throw new Error("Unknown variant index for TransactionPayload: " + index);
  }
}
}


export class TransactionPayloadVariantWriteSet extends TransactionPayload {

constructor (public value: WriteSetPayload) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionPayloadVariantWriteSet {
  const value = WriteSetPayload.deserialize(deserializer);
  return new TransactionPayloadVariantWriteSet(value);
}

}

export class TransactionPayloadVariantScript extends TransactionPayload {

constructor (public value: Script) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionPayloadVariantScript {
  const value = Script.deserialize(deserializer);
  return new TransactionPayloadVariantScript(value);
}

}

export class TransactionPayloadVariantModule extends TransactionPayload {

constructor (public value: Module) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionPayloadVariantModule {
  const value = Module.deserialize(deserializer);
  return new TransactionPayloadVariantModule(value);
}

}

export class TransactionPayloadVariantScriptFunction extends TransactionPayload {

constructor (public value: ScriptFunction) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TransactionPayloadVariantScriptFunction {
  const value = ScriptFunction.deserialize(deserializer);
  return new TransactionPayloadVariantScriptFunction(value);
}

}
export abstract class TravelRuleMetadata {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): TravelRuleMetadata {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TravelRuleMetadataVariantTravelRuleMetadataVersion0.load(deserializer);
    default: throw new Error("Unknown variant index for TravelRuleMetadata: " + index);
  }
}
}


export class TravelRuleMetadataVariantTravelRuleMetadataVersion0 extends TravelRuleMetadata {

constructor (public value: TravelRuleMetadataV0) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TravelRuleMetadataVariantTravelRuleMetadataVersion0 {
  const value = TravelRuleMetadataV0.deserialize(deserializer);
  return new TravelRuleMetadataVariantTravelRuleMetadataVersion0(value);
}

}
export class TravelRuleMetadataV0 {

constructor (public off_chain_reference_id: Optional<str>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeOptionStr(this.off_chain_reference_id, serializer);
}

static deserialize(deserializer: Deserializer): TravelRuleMetadataV0 {
  const off_chain_reference_id = Helpers.deserializeOptionStr(deserializer);
  return new TravelRuleMetadataV0(off_chain_reference_id);
}

}
export abstract class TypeTag {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): TypeTag {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return TypeTagVariantBool.load(deserializer);
    case 1: return TypeTagVariantU8.load(deserializer);
    case 2: return TypeTagVariantU64.load(deserializer);
    case 3: return TypeTagVariantU128.load(deserializer);
    case 4: return TypeTagVariantAddress.load(deserializer);
    case 5: return TypeTagVariantSigner.load(deserializer);
    case 6: return TypeTagVariantVector.load(deserializer);
    case 7: return TypeTagVariantStruct.load(deserializer);
    default: throw new Error("Unknown variant index for TypeTag: " + index);
  }
}
}


export class TypeTagVariantBool extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): TypeTagVariantBool {
  return new TypeTagVariantBool();
}

}

export class TypeTagVariantU8 extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): TypeTagVariantU8 {
  return new TypeTagVariantU8();
}

}

export class TypeTagVariantU64 extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): TypeTagVariantU64 {
  return new TypeTagVariantU64();
}

}

export class TypeTagVariantU128 extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
}

static load(deserializer: Deserializer): TypeTagVariantU128 {
  return new TypeTagVariantU128();
}

}

export class TypeTagVariantAddress extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
}

static load(deserializer: Deserializer): TypeTagVariantAddress {
  return new TypeTagVariantAddress();
}

}

export class TypeTagVariantSigner extends TypeTag {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(5);
}

static load(deserializer: Deserializer): TypeTagVariantSigner {
  return new TypeTagVariantSigner();
}

}

export class TypeTagVariantVector extends TypeTag {

constructor (public value: TypeTag) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(6);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TypeTagVariantVector {
  const value = TypeTag.deserialize(deserializer);
  return new TypeTagVariantVector(value);
}

}

export class TypeTagVariantStruct extends TypeTag {

constructor (public value: StructTag) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(7);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): TypeTagVariantStruct {
  const value = StructTag.deserialize(deserializer);
  return new TypeTagVariantStruct(value);
}

}
export class UnstructuredBytesMetadata {

constructor (public metadata: Optional<bytes>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeOptionBytes(this.metadata, serializer);
}

static deserialize(deserializer: Deserializer): UnstructuredBytesMetadata {
  const metadata = Helpers.deserializeOptionBytes(deserializer);
  return new UnstructuredBytesMetadata(metadata);
}

}
export class VecBytes {

constructor (public value: Seq<bytes>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeVectorBytes(this.value, serializer);
}

static deserialize(deserializer: Deserializer): VecBytes {
  const value = Helpers.deserializeVectorBytes(deserializer);
  return new VecBytes(value);
}

}
export abstract class WriteOp {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): WriteOp {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return WriteOpVariantDeletion.load(deserializer);
    case 1: return WriteOpVariantValue.load(deserializer);
    default: throw new Error("Unknown variant index for WriteOp: " + index);
  }
}
}


export class WriteOpVariantDeletion extends WriteOp {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): WriteOpVariantDeletion {
  return new WriteOpVariantDeletion();
}

}

export class WriteOpVariantValue extends WriteOp {

constructor (public value: bytes) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeBytes(this.value);
}

static load(deserializer: Deserializer): WriteOpVariantValue {
  const value = deserializer.deserializeBytes();
  return new WriteOpVariantValue(value);
}

}
export class WriteSet {

constructor (public value: WriteSetMut) {
}

public serialize(serializer: Serializer): void {
  this.value.serialize(serializer);
}

static deserialize(deserializer: Deserializer): WriteSet {
  const value = WriteSetMut.deserialize(deserializer);
  return new WriteSet(value);
}

}
export class WriteSetMut {

constructor (public write_set: Seq<Tuple<[AccessPath, WriteOp]>>) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeVectorTuple2AccessPathWriteOp(this.write_set, serializer);
}

static deserialize(deserializer: Deserializer): WriteSetMut {
  const write_set = Helpers.deserializeVectorTuple2AccessPathWriteOp(deserializer);
  return new WriteSetMut(write_set);
}

}
export abstract class WriteSetPayload {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): WriteSetPayload {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return WriteSetPayloadVariantDirect.load(deserializer);
    case 1: return WriteSetPayloadVariantScript.load(deserializer);
    default: throw new Error("Unknown variant index for WriteSetPayload: " + index);
  }
}
}


export class WriteSetPayloadVariantDirect extends WriteSetPayload {

constructor (public value: ChangeSet) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): WriteSetPayloadVariantDirect {
  const value = ChangeSet.deserialize(deserializer);
  return new WriteSetPayloadVariantDirect(value);
}

}

export class WriteSetPayloadVariantScript extends WriteSetPayload {

constructor (public execute_as: AccountAddress, public script: Script) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.execute_as.serialize(serializer);
  this.script.serialize(serializer);
}

static load(deserializer: Deserializer): WriteSetPayloadVariantScript {
  const execute_as = AccountAddress.deserialize(deserializer);
  const script = Script.deserialize(deserializer);
  return new WriteSetPayloadVariantScript(execute_as,script);
}

}
export class Helpers {
  static serializeArray16U8Array(value: ListTuple<[uint8]>, serializer: Serializer): void {
    value.forEach((item) =>{
        serializer.serializeU8(item[0]);
    });
  }

  static deserializeArray16U8Array(deserializer: Deserializer): ListTuple<[uint8]> {
    const list: ListTuple<[uint8]> = [];
    for (let i = 0; i < 16; i++) {
        list.push([deserializer.deserializeU8()]);
    }
    return list;
  }

  static serializeOptionBytes(value: Optional<bytes>, serializer: Serializer): void {
    if (value) {
        serializer.serializeOptionTag(true);
        serializer.serializeBytes(value);
    } else {
        serializer.serializeOptionTag(false);
    }
  }

  static deserializeOptionBytes(deserializer: Deserializer): Optional<bytes> {
    const tag = deserializer.deserializeOptionTag();
    if (!tag) {
        return null;
    } else {
        return deserializer.deserializeBytes();
    }
  }

  static serializeOptionStr(value: Optional<str>, serializer: Serializer): void {
    if (value) {
        serializer.serializeOptionTag(true);
        serializer.serializeStr(value);
    } else {
        serializer.serializeOptionTag(false);
    }
  }

  static deserializeOptionStr(deserializer: Deserializer): Optional<str> {
    const tag = deserializer.deserializeOptionTag();
    if (!tag) {
        return null;
    } else {
        return deserializer.deserializeStr();
    }
  }

  static serializeOptionU64(value: Optional<uint64>, serializer: Serializer): void {
    if (value) {
        serializer.serializeOptionTag(true);
        serializer.serializeU64(value);
    } else {
        serializer.serializeOptionTag(false);
    }
  }

  static deserializeOptionU64(deserializer: Deserializer): Optional<uint64> {
    const tag = deserializer.deserializeOptionTag();
    if (!tag) {
        return null;
    } else {
        return deserializer.deserializeU64();
    }
  }

  static serializeTuple2AccessPathWriteOp(value: Tuple<[AccessPath, WriteOp]>, serializer: Serializer): void {
    value[0].serialize(serializer);
    value[1].serialize(serializer);
  }

  static deserializeTuple2AccessPathWriteOp(deserializer: Deserializer): Tuple<[AccessPath, WriteOp]> {
    return [
        AccessPath.deserialize(deserializer),
        WriteOp.deserialize(deserializer)
    ];
  }

  static serializeVectorAccountAddress(value: Seq<AccountAddress>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: AccountAddress) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorAccountAddress(deserializer: Deserializer): Seq<AccountAddress> {
    const length = deserializer.deserializeLen();
    const list: Seq<AccountAddress> = [];
    for (let i = 0; i < length; i++) {
        list.push(AccountAddress.deserialize(deserializer));
    }
    return list;
  }

  static serializeVectorAccountAuthenticator(value: Seq<AccountAuthenticator>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: AccountAuthenticator) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorAccountAuthenticator(deserializer: Deserializer): Seq<AccountAuthenticator> {
    const length = deserializer.deserializeLen();
    const list: Seq<AccountAuthenticator> = [];
    for (let i = 0; i < length; i++) {
        list.push(AccountAuthenticator.deserialize(deserializer));
    }
    return list;
  }

  static serializeVectorContractEvent(value: Seq<ContractEvent>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: ContractEvent) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorContractEvent(deserializer: Deserializer): Seq<ContractEvent> {
    const length = deserializer.deserializeLen();
    const list: Seq<ContractEvent> = [];
    for (let i = 0; i < length; i++) {
        list.push(ContractEvent.deserialize(deserializer));
    }
    return list;
  }

  static serializeVectorTransactionArgument(value: Seq<TransactionArgument>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: TransactionArgument) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorTransactionArgument(deserializer: Deserializer): Seq<TransactionArgument> {
    const length = deserializer.deserializeLen();
    const list: Seq<TransactionArgument> = [];
    for (let i = 0; i < length; i++) {
        list.push(TransactionArgument.deserialize(deserializer));
    }
    return list;
  }

  static serializeVectorTypeTag(value: Seq<TypeTag>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: TypeTag) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorTypeTag(deserializer: Deserializer): Seq<TypeTag> {
    const length = deserializer.deserializeLen();
    const list: Seq<TypeTag> = [];
    for (let i = 0; i < length; i++) {
        list.push(TypeTag.deserialize(deserializer));
    }
    return list;
  }

  static serializeVectorBytes(value: Seq<bytes>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: bytes) => {
        serializer.serializeBytes(item);
    });
  }

  static deserializeVectorBytes(deserializer: Deserializer): Seq<bytes> {
    const length = deserializer.deserializeLen();
    const list: Seq<bytes> = [];
    for (let i = 0; i < length; i++) {
        list.push(deserializer.deserializeBytes());
    }
    return list;
  }

  static serializeVectorStr(value: Seq<str>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: str) => {
        serializer.serializeStr(item);
    });
  }

  static deserializeVectorStr(deserializer: Deserializer): Seq<str> {
    const length = deserializer.deserializeLen();
    const list: Seq<str> = [];
    for (let i = 0; i < length; i++) {
        list.push(deserializer.deserializeStr());
    }
    return list;
  }

  static serializeVectorTuple2AccessPathWriteOp(value: Seq<Tuple<[AccessPath, WriteOp]>>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: Tuple<[AccessPath, WriteOp]>) => {
        Helpers.serializeTuple2AccessPathWriteOp(item, serializer);
    });
  }

  static deserializeVectorTuple2AccessPathWriteOp(deserializer: Deserializer): Seq<Tuple<[AccessPath, WriteOp]>> {
    const length = deserializer.deserializeLen();
    const list: Seq<Tuple<[AccessPath, WriteOp]>> = [];
    for (let i = 0; i < length; i++) {
        list.push(Helpers.deserializeTuple2AccessPathWriteOp(deserializer));
    }
    return list;
  }

}
