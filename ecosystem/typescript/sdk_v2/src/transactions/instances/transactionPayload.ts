/* eslint-disable @typescript-eslint/naming-convention */

import { Serializer, Deserializer, Serializable } from "../../bcs";
import { AccountAddress } from "../../core";
import { Identifier } from "./identifier";
import { ScriptTransactionArgument } from "./scriptTransactionArguments";
import { ModuleId } from "./moduleId";
import { TransactionPayloadVariants } from "../../types";
import { TypeTag } from "../typeTag/typeTag";
import { U8 } from "../../bcs/serializable/move-primitives";
import { MoveVector } from "../../bcs/serializable/move-structs";
import { FixedBytes } from "../../bcs/serializable/fixed-bytes";

/**
 * Representation of the supported Transaction Payload
 * that can serialized and deserialized
 */
export abstract class TransactionPayload extends Serializable {
  /**
   * Serialize a Transaction Payload
   */
  abstract serialize(serializer: Serializer): void;

  /**
   * Deserialize a Transaction Payload
   */
  static deserialize(deserializer: Deserializer): TransactionPayload {
    // index enum variant
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case TransactionPayloadVariants.Script:
        return TransactionPayloadScript.load(deserializer);
      case TransactionPayloadVariants.EntryFunction:
        return TransactionPayloadEntryFunction.load(deserializer);
      case TransactionPayloadVariants.Multisig:
        return TransactionPayloadMultisig.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionPayload: ${index}`);
    }
  }
}

/**
 * Representation of a Transaction Payload Script that can serialized and deserialized
 */
export class TransactionPayloadScript extends TransactionPayload {
  public readonly script: Script;

  constructor(script: Script) {
    super();
    this.script = script;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionPayloadVariants.Script);
    this.script.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadScript {
    const script = Script.deserialize(deserializer);
    return new TransactionPayloadScript(script);
  }
}

/**
 * Representation of a Transaction Payload Entry Function that can serialized and deserialized
 */
export class TransactionPayloadEntryFunction extends TransactionPayload {
  public readonly entryFunction: EntryFunction;

  constructor(entryFunction: EntryFunction) {
    super();
    this.entryFunction = entryFunction;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionPayloadVariants.EntryFunction);
    this.entryFunction.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadEntryFunction {
    const entryFunction = EntryFunction.deserialize(deserializer);
    return new TransactionPayloadEntryFunction(entryFunction);
  }
}

/**
 * Representation of a Transaction Payload Multisig that can serialized and deserialized
 */
export class TransactionPayloadMultisig extends TransactionPayload {
  public readonly multiSig: MultiSig;

  constructor(multiSig: MultiSig) {
    super();
    this.multiSig = multiSig;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionPayloadVariants.Multisig);
    this.multiSig.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadMultisig {
    const multiSig = MultiSig.deserialize(deserializer);
    return new TransactionPayloadMultisig(multiSig);
  }
}

/**
 * Representation of a EntryFunction that can serialized and deserialized
 */
export class EntryFunction {
  public readonly module_name: ModuleId;

  public readonly function_name: Identifier;

  public readonly type_args: Array<TypeTag>;

  public readonly args: Array<Serializable>;

  /**
   * Contains the payload to run a function within a module.
   * @param module_name Fully qualified module name in format "account_address::module_name" e.g. "0x1::coin"
   * @param function_name The function name. e.g "transfer"
   * @param type_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
   * ```
   * @param args arguments to the move function.
   *
   * @example
   * A coin transfer function has three arguments "from", "to" and "amount".
   * ```
   * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
   * ```
   */
  constructor(module_name: ModuleId, function_name: Identifier, type_args: Array<TypeTag>, args: Array<Serializable>) {
    this.module_name = module_name;
    this.function_name = function_name;
    this.type_args = type_args;
    this.args = args;
  }

  /**
   * A helper function to build a EntryFunction payload from raw primitive values
   *
   * @param module_name Fully qualified module name in format "AccountAddress::module_name" e.g. "0x1::coin"
   * @param function_name Function name
   * @param type_args Type arguments that move function requires.
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
   * @returns EntryFunction
   */
  static build(
    module_name: `${string}::${string}`,
    function_name: string,
    type_args: Array<TypeTag>,
    args: Array<Serializable>,
  ): EntryFunction {
    return new EntryFunction(ModuleId.fromStr(module_name), new Identifier(function_name), type_args, args);
  }

  serialize(serializer: Serializer): void {
    this.module_name.serialize(serializer);
    this.function_name.serialize(serializer);
    serializer.serializeVector<TypeTag>(this.type_args);

    serializer.serializeU32AsUleb128(this.args.length);
    this.args.forEach((item: Serializable) => {
      const bytes = item.bcsToBytes();
      serializer.serializeBytes(bytes);
    });
  }

  /**
   * Deserializes an entry function payload with the arguments represented as FixedBytes instances.
   * @see FixedBytes
   *
   * NOTE: When you deserialize an EntryFunction payload with this method, the entry function
   * arguments are populated as type-agnostic, raw fixed bytes in the form of the FixedBytes class.
   * In order to correctly deserialize these arguments as their actual type representations, you
   * must know the types of the arguments beforehand and deserialize them yourself individually.
   *
   * One way you could achieve this is by using the ABIs for an entry function and deserializing each
   * argument as its given, corresponding type.
   *
   * @param deserializer
   * @returns A deserialized EntryFunction payload for a transaction.
   *
   */
  static deserialize(deserializer: Deserializer): EntryFunction {
    const module_name = ModuleId.deserialize(deserializer);
    const function_name = Identifier.deserialize(deserializer);
    const type_args = deserializer.deserializeVector(TypeTag);

    const length = deserializer.deserializeUleb128AsU32();
    const list: Array<Serializable> = new Array<MoveVector<U8>>();

    for (let i = 0; i < length; i += 1) {
      const fixedBytesLength = deserializer.deserializeUleb128AsU32();
      const fixedBytes = FixedBytes.deserialize(deserializer, fixedBytesLength);
      list.push(fixedBytes);
    }

    const args = list;

    return new EntryFunction(module_name, function_name, type_args, args);
  }
}

/**
 * Representation of a Script that can serialized and deserialized
 */
export class Script {
  /**
   * The move module bytecode
   */
  public readonly bytecode: Uint8Array;

  /**
   * The type arguments that the bytecode function requires.
   */
  public readonly type_args: Array<TypeTag>;

  /**
   * The arguments that the bytecode function requires.
   */
  public readonly args: Array<ScriptTransactionArgument>;

  /**
   * Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.
   *
   * @param code The move module bytecode
   * @param type_args The type arguments that the bytecode function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args The arguments that the bytecode function requires.
   *
   * @example
   * A coin transfer function has three arguments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(bytecode: Uint8Array, type_args: Array<TypeTag>, args: Array<ScriptTransactionArgument>) {
    this.bytecode = bytecode;
    this.type_args = type_args;
    this.args = args;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.bytecode);
    serializer.serializeVector<TypeTag>(this.type_args);
    serializer.serializeVector<ScriptTransactionArgument>(this.args);
  }

  static deserialize(deserializer: Deserializer): Script {
    const bytecode = deserializer.deserializeBytes();
    const type_args = deserializer.deserializeVector(TypeTag);
    const args = deserializer.deserializeVector(ScriptTransactionArgument);
    return new Script(bytecode, type_args, args);
  }
}

/**
 * Representation of a MultiSig that can serialized and deserialized
 */
export class MultiSig {
  public readonly multisig_address: AccountAddress;

  public readonly transaction_payload?: MultiSigTransactionPayload;

  /**
   * Contains the payload to run a multisig account transaction.
   *
   * @param multisig_address The multisig account address the transaction will be executed as.
   *
   * @param transaction_payload The payload of the multisig transaction. This is optional when executing a multisig
   *  transaction whose payload is already stored on chain.
   */
  constructor(multisig_address: AccountAddress, transaction_payload?: MultiSigTransactionPayload) {
    this.multisig_address = multisig_address;
    this.transaction_payload = transaction_payload;
  }

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

/**
 * Representation of a MultiSig Transaction Payload that can serialized and deserialized
 */
export class MultiSigTransactionPayload {
  public readonly transaction_payload: EntryFunction;

  /**
   * Contains the payload to run a multisig account transaction.
   *
   * @param transaction_payload The payload of the multisig transaction.
   * This can only be EntryFunction for now but,
   * Script might be supported in the future.
   */
  constructor(transaction_payload: EntryFunction) {
    this.transaction_payload = transaction_payload;
  }

  serialize(serializer: Serializer): void {
    /**
     * We can support multiple types of inner transaction payload in the future.
     * For now it's only EntryFunction but if we support more types,
     * we need to serialize with the right enum values here
     */
    serializer.serializeU32AsUleb128(0);
    this.transaction_payload.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): MultiSigTransactionPayload {
    // This is the enum value indicating which type of payload the multisig tx contains.
    deserializer.deserializeUleb128AsU32();
    return new MultiSigTransactionPayload(EntryFunction.deserialize(deserializer));
  }
}
