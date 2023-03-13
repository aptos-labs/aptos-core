// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable class-methods-use-this */
/* eslint-disable @typescript-eslint/no-unused-vars */
/* eslint-disable max-classes-per-file */
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { HexString } from "../utils";
import {
  Deserializer,
  Serializer,
  Uint64,
  Bytes,
  Seq,
  Uint8,
  Uint128,
  deserializeVector,
  serializeVector,
  bcsToBytes,
  Uint16,
  Uint256,
} from "../bcs";
import { TransactionAuthenticator } from "./authenticator";
import { Identifier } from "./identifier";
import { TypeTag } from "./type_tag";
import { AccountAddress } from "./account_address";

export class RawTransaction {
  /**
   * RawTransactions contain the metadata and payloads that can be submitted to Aptos chain for execution.
   * RawTransactions must be signed before Aptos chain can execute them.
   *
   * @param sender Account address of the sender.
   * @param sequence_number Sequence number of this transaction. This must match the sequence number stored in
   *   the sender's account at the time the transaction executes.
   * @param payload Instructions for the Aptos Blockchain, including publishing a module,
   *   execute a entry function or execute a script payload.
   * @param max_gas_amount Maximum total gas to spend for this transaction. The account must have more
   *   than this gas or the transaction will be discarded during validation.
   * @param gas_unit_price Price to be paid per gas unit.
   * @param expiration_timestamp_secs The blockchain timestamp at which the blockchain would discard this transaction.
   * @param chain_id The chain ID of the blockchain that this transaction is intended to be run on.
   */
  constructor(
    public readonly sender: AccountAddress,
    public readonly sequence_number: Uint64,
    public readonly payload: TransactionPayload,
    public readonly max_gas_amount: Uint64,
    public readonly gas_unit_price: Uint64,
    public readonly expiration_timestamp_secs: Uint64,
    public readonly chain_id: ChainId,
  ) {}

  serialize(serializer: Serializer): void {
    this.sender.serialize(serializer);
    serializer.serializeU64(this.sequence_number);
    this.payload.serialize(serializer);
    serializer.serializeU64(this.max_gas_amount);
    serializer.serializeU64(this.gas_unit_price);
    serializer.serializeU64(this.expiration_timestamp_secs);
    this.chain_id.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): RawTransaction {
    const sender = AccountAddress.deserialize(deserializer);
    const sequence_number = deserializer.deserializeU64();
    const payload = TransactionPayload.deserialize(deserializer);
    const max_gas_amount = deserializer.deserializeU64();
    const gas_unit_price = deserializer.deserializeU64();
    const expiration_timestamp_secs = deserializer.deserializeU64();
    const chain_id = ChainId.deserialize(deserializer);
    return new RawTransaction(
      sender,
      sequence_number,
      payload,
      max_gas_amount,
      gas_unit_price,
      expiration_timestamp_secs,
      chain_id,
    );
  }
}

export class Script {
  /**
   * Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.
   * @param code Move bytecode
   * @param ty_args Type arguments that bytecode requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to bytecode function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(
    public readonly code: Bytes,
    public readonly ty_args: Seq<TypeTag>,
    public readonly args: Seq<TransactionArgument>,
  ) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.code);
    serializeVector<TypeTag>(this.ty_args, serializer);
    serializeVector<TransactionArgument>(this.args, serializer);
  }

  static deserialize(deserializer: Deserializer): Script {
    const code = deserializer.deserializeBytes();
    const ty_args = deserializeVector(deserializer, TypeTag);
    const args = deserializeVector(deserializer, TransactionArgument);
    return new Script(code, ty_args, args);
  }
}

export class EntryFunction {
  /**
   * Contains the payload to run a function within a module.
   * @param module_name Fully qualified module name. ModuleId consists of account address and module name.
   * @param function_name The function to run.
   * @param ty_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to the move function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(
    public readonly module_name: ModuleId,
    public readonly function_name: Identifier,
    public readonly ty_args: Seq<TypeTag>,
    public readonly args: Seq<Bytes>,
  ) {}

  /**
   *
   * @param module Fully qualified module name in format "AccountAddress::module_name" e.g. "0x1::coin"
   * @param func Function name
   * @param ty_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to the move function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @returns
   */
  static natural(module: string, func: string, ty_args: Seq<TypeTag>, args: Seq<Bytes>): EntryFunction {
    return new EntryFunction(ModuleId.fromStr(module), new Identifier(func), ty_args, args);
  }

  /**
   * `natual` is deprecated, please use `natural`
   *
   * @deprecated.
   */
  static natual(module: string, func: string, ty_args: Seq<TypeTag>, args: Seq<Bytes>): EntryFunction {
    return EntryFunction.natural(module, func, ty_args, args);
  }

  serialize(serializer: Serializer): void {
    this.module_name.serialize(serializer);
    this.function_name.serialize(serializer);
    serializeVector<TypeTag>(this.ty_args, serializer);

    serializer.serializeU32AsUleb128(this.args.length);
    this.args.forEach((item: Bytes) => {
      serializer.serializeBytes(item);
    });
  }

  static deserialize(deserializer: Deserializer): EntryFunction {
    const module_name = ModuleId.deserialize(deserializer);
    const function_name = Identifier.deserialize(deserializer);
    const ty_args = deserializeVector(deserializer, TypeTag);

    const length = deserializer.deserializeUleb128AsU32();
    const list: Seq<Bytes> = [];
    for (let i = 0; i < length; i += 1) {
      list.push(deserializer.deserializeBytes());
    }

    const args = list;
    return new EntryFunction(module_name, function_name, ty_args, args);
  }
}

export class MultiSigTransactionPayload {
  /**
   * Contains the payload to run a multisig account transaction.
   * @param transaction_payload The payload of the multisig transaction. This can only be EntryFunction for now but
   * Script might be supported in the future.
   */
  constructor(public readonly transaction_payload: EntryFunction) {}

  serialize(serializer: Serializer): void {
    // We can support multiple types of inner transaction payload in the future.
    // For now it's only EntryFunction but if we support more types, we need to serialize with the right enum values
    // here
    serializer.serializeU32AsUleb128(0);
    this.transaction_payload.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): MultiSigTransactionPayload {
    return new MultiSigTransactionPayload(EntryFunction.deserialize(deserializer));
  }
}

export class MultiSig {
  /**
   * Contains the payload to run a multisig account transaction.
   * @param multisig_address The multisig account address the transaction will be executed as.
   * @param transaction_payload The payload of the multisig transaction. This is optional when executing a multisig
   *  transaction whose payload is already stored on chain.
   */
  constructor(
    public readonly multisig_address: AccountAddress,
    public readonly transaction_payload?: MultiSigTransactionPayload,
  ) {}

  serialize(serializer: Serializer): void {
    this.multisig_address.serialize(serializer);
    // Options are encoded with an extra u8 field before the value - 0x0 is none and 0x1 is present.
    // We use serializeBool below to create this prefix value.
    if (this.transaction_payload === undefined) {
      serializer.serializeBool(false);
    } else {
      serializer.serializeBool(true);
      this.transaction_payload.serialize(serializer);
    }
  }

  static deserialize(deserializer: Deserializer): MultiSig {
    const multisig_address = AccountAddress.deserialize(deserializer);
    const payloadPresent = deserializer.deserializeBool();
    let transaction_payload;
    if (payloadPresent) {
      transaction_payload = MultiSigTransactionPayload.deserialize(deserializer);
    }
    return new MultiSig(multisig_address, transaction_payload);
  }
}

export class Module {
  /**
   * Contains the bytecode of a Move module that can be published to the Aptos chain.
   * @param code Move bytecode of a module.
   */
  constructor(public readonly code: Bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.code);
  }

  static deserialize(deserializer: Deserializer): Module {
    const code = deserializer.deserializeBytes();
    return new Module(code);
  }
}

export class ModuleId {
  /**
   * Full name of a module.
   * @param address The account address.
   * @param name The name of the module under the account at "address".
   */
  constructor(public readonly address: AccountAddress, public readonly name: Identifier) {}

  /**
   * Converts a string literal to a ModuleId
   * @param moduleId String literal in format "AccountAddress::module_name",
   *   e.g. "0x1::coin"
   * @returns
   */
  static fromStr(moduleId: string): ModuleId {
    const parts = moduleId.split("::");
    if (parts.length !== 2) {
      throw new Error("Invalid module id.");
    }
    return new ModuleId(AccountAddress.fromHex(new HexString(parts[0])), new Identifier(parts[1]));
  }

  serialize(serializer: Serializer): void {
    this.address.serialize(serializer);
    this.name.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): ModuleId {
    const address = AccountAddress.deserialize(deserializer);
    const name = Identifier.deserialize(deserializer);
    return new ModuleId(address, name);
  }
}

export class ChangeSet {
  serialize(serializer: Serializer): void {
    throw new Error("Not implemented.");
  }

  static deserialize(deserializer: Deserializer): ChangeSet {
    throw new Error("Not implemented.");
  }
}

export class WriteSet {
  serialize(serializer: Serializer): void {
    throw new Error("Not implmented.");
  }

  static deserialize(deserializer: Deserializer): WriteSet {
    throw new Error("Not implmented.");
  }
}

export class SignedTransaction {
  /**
   * A SignedTransaction consists of a raw transaction and an authenticator. The authenticator
   * contains a client's public key and the signature of the raw transaction.
   *
   * @see {@link https://aptos.dev/guides/creating-a-signed-transaction/ | Creating a Signed Transaction}
   *
   * @param raw_txn
   * @param authenticator Contains a client's public key and the signature of the raw transaction.
   *   Authenticator has 3 flavors: single signature, multi-signature and multi-agent.
   *   @see authenticator.ts for details.
   */
  constructor(public readonly raw_txn: RawTransaction, public readonly authenticator: TransactionAuthenticator) {}

  serialize(serializer: Serializer): void {
    this.raw_txn.serialize(serializer);
    this.authenticator.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): SignedTransaction {
    const raw_txn = RawTransaction.deserialize(deserializer);
    const authenticator = TransactionAuthenticator.deserialize(deserializer);
    return new SignedTransaction(raw_txn, authenticator);
  }
}

export abstract class RawTransactionWithData {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): RawTransactionWithData {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return MultiAgentRawTransaction.load(deserializer);
      default:
        throw new Error(`Unknown variant index for RawTransactionWithData: ${index}`);
    }
  }
}

export class MultiAgentRawTransaction extends RawTransactionWithData {
  constructor(
    public readonly raw_txn: RawTransaction,
    public readonly secondary_signer_addresses: Seq<AccountAddress>,
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    // enum variant index
    serializer.serializeU32AsUleb128(0);
    this.raw_txn.serialize(serializer);
    serializeVector<TransactionArgument>(this.secondary_signer_addresses, serializer);
  }

  static load(deserializer: Deserializer): MultiAgentRawTransaction {
    const rawTxn = RawTransaction.deserialize(deserializer);
    const secondarySignerAddresses = deserializeVector(deserializer, AccountAddress);

    return new MultiAgentRawTransaction(rawTxn, secondarySignerAddresses);
  }
}

export abstract class TransactionPayload {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionPayload {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionPayloadScript.load(deserializer);
      // TODO: change to 1 once ModuleBundle has been removed from rust
      case 2:
        return TransactionPayloadEntryFunction.load(deserializer);
      case 3:
        return TransactionPayloadMultisig.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionPayload: ${index}`);
    }
  }
}

export class TransactionPayloadScript extends TransactionPayload {
  constructor(public readonly value: Script) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadScript {
    const value = Script.deserialize(deserializer);
    return new TransactionPayloadScript(value);
  }
}

export class TransactionPayloadEntryFunction extends TransactionPayload {
  constructor(public readonly value: EntryFunction) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadEntryFunction {
    const value = EntryFunction.deserialize(deserializer);
    return new TransactionPayloadEntryFunction(value);
  }
}

export class TransactionPayloadMultisig extends TransactionPayload {
  constructor(public readonly value: MultiSig) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadMultisig {
    const value = MultiSig.deserialize(deserializer);
    return new TransactionPayloadMultisig(value);
  }
}

export class ChainId {
  constructor(public readonly value: Uint8) {}

  serialize(serializer: Serializer): void {
    serializer.serializeU8(this.value);
  }

  static deserialize(deserializer: Deserializer): ChainId {
    const value = deserializer.deserializeU8();
    return new ChainId(value);
  }
}

export abstract class TransactionArgument {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionArgument {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionArgumentU8.load(deserializer);
      case 1:
        return TransactionArgumentU64.load(deserializer);
      case 2:
        return TransactionArgumentU128.load(deserializer);
      case 3:
        return TransactionArgumentAddress.load(deserializer);
      case 4:
        return TransactionArgumentU8Vector.load(deserializer);
      case 5:
        return TransactionArgumentBool.load(deserializer);
      case 6:
        return TransactionArgumentU16.load(deserializer);
      case 7:
        return TransactionArgumentU32.load(deserializer);
      case 8:
        return TransactionArgumentU256.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionArgument: ${index}`);
    }
  }
}

export class TransactionArgumentU8 extends TransactionArgument {
  constructor(public readonly value: Uint8) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    serializer.serializeU8(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU8 {
    const value = deserializer.deserializeU8();
    return new TransactionArgumentU8(value);
  }
}

export class TransactionArgumentU16 extends TransactionArgument {
  constructor(public readonly value: Uint16) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(6);
    serializer.serializeU16(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU16 {
    const value = deserializer.deserializeU16();
    return new TransactionArgumentU16(value);
  }
}

export class TransactionArgumentU32 extends TransactionArgument {
  constructor(public readonly value: Uint16) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(7);
    serializer.serializeU32(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU32 {
    const value = deserializer.deserializeU32();
    return new TransactionArgumentU32(value);
  }
}

export class TransactionArgumentU64 extends TransactionArgument {
  constructor(public readonly value: Uint64) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    serializer.serializeU64(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU64 {
    const value = deserializer.deserializeU64();
    return new TransactionArgumentU64(value);
  }
}

export class TransactionArgumentU128 extends TransactionArgument {
  constructor(public readonly value: Uint128) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    serializer.serializeU128(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU128 {
    const value = deserializer.deserializeU128();
    return new TransactionArgumentU128(value);
  }
}

export class TransactionArgumentU256 extends TransactionArgument {
  constructor(public readonly value: Uint256) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(8);
    serializer.serializeU256(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU256 {
    const value = deserializer.deserializeU256();
    return new TransactionArgumentU256(value);
  }
}

export class TransactionArgumentAddress extends TransactionArgument {
  constructor(public readonly value: AccountAddress) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionArgumentAddress {
    const value = AccountAddress.deserialize(deserializer);
    return new TransactionArgumentAddress(value);
  }
}

export class TransactionArgumentU8Vector extends TransactionArgument {
  constructor(public readonly value: Bytes) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(4);
    serializer.serializeBytes(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU8Vector {
    const value = deserializer.deserializeBytes();
    return new TransactionArgumentU8Vector(value);
  }
}

export class TransactionArgumentBool extends TransactionArgument {
  constructor(public readonly value: boolean) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(5);
    serializer.serializeBool(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentBool {
    const value = deserializer.deserializeBool();
    return new TransactionArgumentBool(value);
  }
}

export abstract class Transaction {
  abstract serialize(serializer: Serializer): void;

  abstract hash(): Bytes;

  getHashSalt(): Bytes {
    const hash = sha3Hash.create();
    hash.update("APTOS::Transaction");
    return hash.digest();
  }

  static deserialize(deserializer: Deserializer): Transaction {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return UserTransaction.load(deserializer);
      default:
        throw new Error(`Unknown variant index for Transaction: ${index}`);
    }
  }
}

export class UserTransaction extends Transaction {
  constructor(public readonly value: SignedTransaction) {
    super();
  }

  hash(): Bytes {
    const hash = sha3Hash.create();
    hash.update(this.getHashSalt());
    hash.update(bcsToBytes(this));
    return hash.digest();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): UserTransaction {
    return new UserTransaction(SignedTransaction.deserialize(deserializer));
  }
}
