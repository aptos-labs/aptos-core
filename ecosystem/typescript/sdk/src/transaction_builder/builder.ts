import * as SHA3 from "js-sha3";
import { Buffer } from "buffer/";
import {
  Ed25519PublicKey,
  Ed25519Signature,
  MultiEd25519PublicKey,
  MultiEd25519Signature,
  RawTransaction,
  SignedTransaction,
  TransactionAuthenticatorEd25519,
  TransactionAuthenticatorMultiEd25519,
  SigningMessage,
  MultiAgentRawTransaction,
  AccountAddress,
  ScriptFunction,
  Identifier,
  ChainId,
  Script,
  TransactionPayload,
  TransactionArgument,
  TransactionPayloadScriptFunction,
  TransactionPayloadScript,
} from "./aptos_types";
import { bcsToBytes, Bytes, Deserializer, Serializer, Uint64, Uint8 } from "./bcs";
import { ScriptABI, ScriptFunctionABI, TransactionScriptABI } from "./aptos_types/abi";
import { HexString } from "../hex_string";
import { argToTransactionArgument, TypeTagParser, serializeArg } from "./builder_utils";

const RAW_TRANSACTION_SALT = "APTOS::RawTransaction";
const RAW_TRANSACTION_WITH_DATA_SALT = "APTOS::RawTransactionWithData";

type AnyRawTransaction = RawTransaction | MultiAgentRawTransaction;

/**
 * Function that takes in a Signing Message (serialized raw transaction)
 *  and returns a signature
 */
export type SigningFn = (txn: SigningMessage) => Ed25519Signature | MultiEd25519Signature;

export class TransactionBuilder<F extends SigningFn> {
  protected readonly signingFunction: F;

  constructor(signingFunction: F, public readonly rawTxnBuilder?: TransactionBuilderABI) {
    this.signingFunction = signingFunction;
  }

  /**
   * Builds a RawTransaction. Relays the call to TransactionBuilderABI.build
   * @param func
   * @param ty_tags
   * @param args
   */
  build(func: string, ty_tags: string[], args: any[]): RawTransaction {
    if (!this.rawTxnBuilder) {
      throw new Error("this.rawTxnBuilder doesn't exist.");
    }

    return this.rawTxnBuilder.build(func, ty_tags, args);
  }

  /** Generates a Signing Message out of a raw transaction. */
  static getSigningMessage(rawTxn: AnyRawTransaction): SigningMessage {
    const hash = SHA3.sha3_256.create();
    if (rawTxn instanceof RawTransaction) {
      hash.update(Buffer.from(RAW_TRANSACTION_SALT));
    } else if (rawTxn instanceof MultiAgentRawTransaction) {
      hash.update(Buffer.from(RAW_TRANSACTION_WITH_DATA_SALT));
    } else {
      throw new Error("Unknown transaction type.");
    }

    const prefix = new Uint8Array(hash.arrayBuffer());

    return Buffer.from([...prefix, ...bcsToBytes(rawTxn)]);
  }
}

/**
 * Provides signing method for signing a raw transaction with single public key.
 */
export class TransactionBuilderEd25519 extends TransactionBuilder<SigningFn> {
  private readonly publicKey: Uint8Array;

  constructor(signingFunction: SigningFn, publicKey: Uint8Array, rawTxnBuilder?: TransactionBuilderABI) {
    super(signingFunction, rawTxnBuilder);
    this.publicKey = publicKey;
  }

  rawToSigned(rawTxn: RawTransaction): SignedTransaction {
    const signingMessage = TransactionBuilder.getSigningMessage(rawTxn);
    const signature = this.signingFunction(signingMessage);

    const authenticator = new TransactionAuthenticatorEd25519(
      new Ed25519PublicKey(this.publicKey),
      signature as Ed25519Signature,
    );

    return new SignedTransaction(rawTxn, authenticator);
  }

  /** Signs a raw transaction and returns a bcs serialized transaction. */
  sign(rawTxn: RawTransaction): Bytes {
    return bcsToBytes(this.rawToSigned(rawTxn));
  }
}

/**
 * Provides signing method for signing a raw transaction with multisig public key.
 */
export class TransactionBuilderMultiEd25519 extends TransactionBuilder<SigningFn> {
  private readonly publicKey: MultiEd25519PublicKey;

  constructor(signingFunction: SigningFn, publicKey: MultiEd25519PublicKey) {
    super(signingFunction);
    this.publicKey = publicKey;
  }

  rawToSigned(rawTxn: RawTransaction): SignedTransaction {
    const signingMessage = TransactionBuilder.getSigningMessage(rawTxn);
    const signature = this.signingFunction(signingMessage);

    const authenticator = new TransactionAuthenticatorMultiEd25519(this.publicKey, signature as MultiEd25519Signature);

    return new SignedTransaction(rawTxn, authenticator);
  }

  /** Signs a raw transaction and returns a bcs serialized transaction. */
  sign(rawTxn: RawTransaction): Bytes {
    return bcsToBytes(this.rawToSigned(rawTxn));
  }
}

/**
 * Config for creating raw transactions.
 */
interface ABIBuilderConfig {
  sender: HexString | AccountAddress;
  sequenceNumber: Uint64 | string;
  gasUnitPrice?: Uint64 | string;
  maxGasAmount?: Uint64 | string;
  expSecFromNow?: number | string;
  chainId: Uint8 | string;
}

/**
 * Builds raw transactions based on ABI
 */
export class TransactionBuilderABI {
  private readonly abiMap: Map<string, ScriptABI>;

  private readonly builderConfig: ABIBuilderConfig;

  /**
   * Constructs a TransactionBuilderABI instance
   * @param abis List of binary ABIs.
   * @param builderConfig Configs for creating a raw transaction.
   */
  constructor(abis: Bytes[], builderConfig: ABIBuilderConfig) {
    this.abiMap = new Map<string, ScriptABI>();

    abis.forEach((abi) => {
      const deserializer = new Deserializer(abi);
      const scriptABI = ScriptABI.deserialize(deserializer);
      let k: string;
      if (scriptABI instanceof ScriptFunctionABI) {
        const funcABI = scriptABI as ScriptFunctionABI;
        const { address: addr, name: moduleName } = funcABI.module_name;
        k = `${HexString.fromUint8Array(addr.address).toShortString()}::${moduleName.value}::${funcABI.name}`;
      } else {
        const funcABI = scriptABI as TransactionScriptABI;
        k = funcABI.name;
      }

      if (this.abiMap.has(k)) {
        throw new Error("Found conflicting ABI interfaces");
      }

      this.abiMap.set(k, scriptABI);
    });

    this.builderConfig = {
      gasUnitPrice: 1n,
      maxGasAmount: 1000n,
      expSecFromNow: 10,
      ...builderConfig,
    };
  }

  private static toBCSArgs(abiArgs: any[], args: any[]): Bytes[] {
    if (abiArgs.length !== args.length) {
      throw new Error("Wrong number of args provided.");
    }

    return args.map((arg, i) => {
      const serializer = new Serializer();
      serializeArg(arg, abiArgs[i].type_tag, serializer);
      return serializer.getBytes();
    });
  }

  private static toTransactionArguments(abiArgs: any[], args: any[]): TransactionArgument[] {
    if (abiArgs.length !== args.length) {
      throw new Error("Wrong number of args provided.");
    }

    return args.map((arg, i) => argToTransactionArgument(arg, abiArgs[i].type_tag));
  }

  setSequenceNumber(seqNumber: Uint64 | string) {
    this.builderConfig.sequenceNumber = BigInt(seqNumber);
  }

  /**
   * Builds a RawTransaction
   * @param func Fully qualified func names, e.g. 0x1::Coin::transfer
   * @param ty_tags TypeTag strings.
   * @example Below are valid value examples
   * ```
   * // Structs are in format `AccountAddress::ModuleName::StructName`
   * 0x1::aptos_coin::AptosCoin
   * // Vectors are in format `vector<other_tag_string>`
   * vector<0x1::aptos_coin::AptosCoin>
   * bool
   * u8
   * u64
   * u128
   * address
   * ```
   * @param args Function arguments
   * @returns RawTransaction
   */
  build(func: string, ty_tags: string[], args: any[]): RawTransaction {
    const { sender, sequenceNumber, gasUnitPrice, maxGasAmount, expSecFromNow, chainId } = this.builderConfig;

    const senderAccount = sender instanceof HexString ? AccountAddress.fromHex(sender) : sender;

    const typeTags = ty_tags.map((ty_arg) => new TypeTagParser(ty_arg).parseTypeTag());

    const expTimetampSec = BigInt(Math.floor(Date.now() / 1000) + Number(expSecFromNow));

    let payload: TransactionPayload;

    if (!this.abiMap.has(func)) {
      throw new Error(`Cannot find function: ${func}`);
    }

    const scriptABI = this.abiMap.get(func);

    if (scriptABI instanceof ScriptFunctionABI) {
      const funcABI = scriptABI as ScriptFunctionABI;
      const bcsArgs = TransactionBuilderABI.toBCSArgs(funcABI.args, args);
      payload = new TransactionPayloadScriptFunction(
        new ScriptFunction(funcABI.module_name, new Identifier(funcABI.name), typeTags, bcsArgs),
      );
    }

    if (scriptABI instanceof TransactionScriptABI) {
      const funcABI = scriptABI as TransactionScriptABI;
      const scriptArgs = TransactionBuilderABI.toTransactionArguments(funcABI.args, args);

      payload = new TransactionPayloadScript(new Script(funcABI.code, typeTags, scriptArgs));
    }

    if (payload) {
      return new RawTransaction(
        senderAccount,
        BigInt(sequenceNumber),
        payload,
        BigInt(maxGasAmount),
        BigInt(gasUnitPrice),
        expTimetampSec,
        new ChainId(Number(chainId)),
      );
    }

    throw new Error("Invalid ABI.");
  }
}
