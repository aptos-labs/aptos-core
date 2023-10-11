/**
 * This file handles the transaction creation lifecycle.
 * It holds different operations to generate a transaction payload, a raw transaciotn,
 * and a signed transaction that can be simulated, signed and submitted to chain.
 */
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { hexToBytes } from "@noble/hashes/utils";
import { AptosConfig } from "../../api/aptos_config";
import { Deserializer } from "../../bcs";
import { AccountAddress } from "../../core";
import { Account } from "../../core/account";
import { Ed25519PublicKey, Ed25519Signature } from "../../crypto/ed25519";
import { Secp256k1PublicKey, Secp256k1Signature } from "../../crypto/secp256k1";
import { getInfo } from "../../internal/account";
import { getLedgerInfo } from "../../internal/general";
import { getGasPriceEstimation } from "../../internal/transaction";
import { HexInput, SigningScheme } from "../../types";
import { NetworkToChainId } from "../../utils/api-endpoints";
import { DEFAULT_MAX_GAS_AMOUNT, DEFAULT_TXN_EXP_SEC_FROM_NOW } from "../../utils/const";
import {
  AccountAuthenticator,
  AccountAuthenticatorEd25519,
  AccountAuthenticatorSecp256k1,
} from "../authenticator/account";
import {
  TransactionAuthenticatorEd25519,
  TransactionAuthenticatorFeePayer,
  TransactionAuthenticatorMultiAgent,
  TransactionAuthenticatorSecp256k1,
} from "../authenticator/transaction";
import {
  ChainId,
  EntryFunction,
  FeePayerRawTransaction,
  MultiAgentRawTransaction,
  MultiSig,
  MultiSigTransactionPayload,
  RawTransaction,
  Script,
  TransactionPayloadEntryFunction,
  TransactionPayloadMultisig,
  TransactionPayloadScript,
} from "../instances";
import { SignedTransaction } from "../instances/signedTransaction";
import {
  GenerateTransactionOptions,
  TransactionPayload,
  AnyRawTransactionInstance,
  GenerateTransactionPayloadData,
  GenerateFeePayerRawTransactionArgs,
  GenerateMultiAgentRawTransactionArgs,
  GenerateRawTransactionArgs,
  GenerateSingleSignerRawTransactionArgs,
  SingleSignerTransaction,
  AnyRawTransaction,
  FeePayerTransaction,
  MultiAgentTransaction,
  EntryFunctionData,
  MultiSigData,
  ScriptData,
  SimulateTransactionData,
} from "../types";

/**
 * We are defining function signatures, each with its specific input and output.
 * These are the possible function signature for our `generateTransactionPayload` function.
 * When we call our `generateTransactionPayload` function with the relevant type properties,
 * Typescript can infer the return type based on the appropriate function overload.
 */
export function generateTransactionPayload(args: EntryFunctionData): TransactionPayloadEntryFunction;
export function generateTransactionPayload(args: ScriptData): TransactionPayloadScript;
export function generateTransactionPayload(args: MultiSigData): TransactionPayloadMultisig;
export function generateTransactionPayload(args: GenerateTransactionPayloadData): TransactionPayload;
/**
 * Builds a transaction payload based on the data argument and returns
 * a transaction payload - TransactionPayloadScript | TransactionPayloadMultisig | TransactionPayloadEntryFunction
 *
 * @param args.data GenerateTransactionPayloadData
 *
 * @return TransactionPayload
 */
export function generateTransactionPayload(args: GenerateTransactionPayloadData): TransactionPayload {
  // generate script payload
  if ("bytecode" in args) {
    const scriptPayload = new TransactionPayloadScript(
      new Script(hexToBytes(args.bytecode), args.type_arguments, args.arguments),
    );
    return scriptPayload;
  }

  // generate multi sig payload
  if ("multisigAddress" in args) {
    const funcNameParts = args.function.split("::");
    const multiSigPayload = new TransactionPayloadMultisig(
      new MultiSig(
        args.multisigAddress,
        new MultiSigTransactionPayload(
          EntryFunction.build(
            `${funcNameParts[0]}::${funcNameParts[1]}`,
            funcNameParts[2],
            args.type_arguments,
            args.arguments,
          ),
        ),
      ),
    );
    return multiSigPayload;
  }

  // generate entry function payload
  const funcNameParts = args.function.split("::");
  const entryFunctionPayload = new TransactionPayloadEntryFunction(
    EntryFunction.build(
      `${funcNameParts[0]}::${funcNameParts[1]}`,
      funcNameParts[2],
      args.type_arguments,
      args.arguments,
    ),
  );
  return entryFunctionPayload;
}

/**
 * Generates a raw transaction
 *
 * @param args.aptosConfig AptosConfig
 * @param args.sendet The transaction's sender account address as a hex input
 * @param args.payload The transaction payload - can create by using generateTransactionPayload()
 *
 * @returns RawTransaction
 */
export async function generateRawTransaction(args: {
  aptosConfig: AptosConfig;
  sender: HexInput;
  payload: TransactionPayload;
  options?: GenerateTransactionOptions;
}): Promise<RawTransaction> {
  const { aptosConfig, sender, payload, options } = args;
  const [{ sequence_number: sequenceNumber }, chainId, { gas_estimate: gasEstimate }] = await Promise.all([
    options?.accountSequenceNumber
      ? Promise.resolve({ sequence_number: options.accountSequenceNumber })
      : await getInfo({ aptosConfig, accountAddress: sender }),
    NetworkToChainId[aptosConfig.network] ?? (await getLedgerInfo({ aptosConfig })).chain_id,
    options?.gasUnitPrice
      ? Promise.resolve({ gas_estimate: options.gasUnitPrice })
      : await getGasPriceEstimation({ aptosConfig }),
  ]);

  const { maxGasAmount, gasUnitPrice, expireTimestamp } = {
    maxGasAmount: BigInt(DEFAULT_MAX_GAS_AMOUNT),
    gasUnitPrice: BigInt(gasEstimate),
    expireTimestamp: BigInt(Math.floor(Date.now() / 1000) + DEFAULT_TXN_EXP_SEC_FROM_NOW),
    ...options,
  };

  return new RawTransaction(
    AccountAddress.fromHexInput({ input: sender }),
    BigInt(sequenceNumber),
    payload,
    BigInt(maxGasAmount),
    BigInt(gasUnitPrice),
    BigInt(expireTimestamp),
    new ChainId(chainId),
  );
}

/**
 * We are defining function signatures, each with its specific input and output.
 * These are the possible function signature for our `generateTransaction` function.
 * When we call our `generateTransaction` function with the relevant type properties,
 * Typescript can infer the return type based on the appropriate function overload.
 */
export async function generateTransaction(
  args: GenerateSingleSignerRawTransactionArgs,
): Promise<SingleSignerTransaction>;
export async function generateTransaction(args: GenerateFeePayerRawTransactionArgs): Promise<FeePayerTransaction>;
export async function generateTransaction(args: GenerateMultiAgentRawTransactionArgs): Promise<MultiAgentTransaction>;
export async function generateTransaction(args: GenerateRawTransactionArgs): Promise<AnyRawTransaction>;
/**
 * Generates a transaction based on the provided arguments
 *
 * Note: we can start with one function to support all different payload/transaction types,
 * and if to complex to use, we could have function for each type
 *
 * @param args.aptosConfig AptosConfig
 * @param args.sendet The transaction's sender account address as a hex input
 * @param args.payload The transaction payload - can create by using generateTransactionPayload()
 * @param args.options optional. Transaction options object
 * @param args.secondarySignerAddresses optional. For when want to create a multi signers transaction
 * @param args.feePayerAddress optional. For when want to create a fee payer (aka sponsored) transaction
 *
 * @return An Aptos raw transaction type (note that it holds the raw transaction as a bcs serialized data)
 * ```
 * {
 *  rawTransaction: Uint8Array,
 *  secondarySignerAddresses? : Array<AccountAddress>,
 *  feePayerAddress?: AccountAddress
 * }
 * ```
 */
export async function generateTransaction(args: GenerateRawTransactionArgs): Promise<AnyRawTransaction> {
  const { aptosConfig, sender, payload, options, secondarySignerAddresses, feePayerAddress } = args;
  // generate raw transaction
  const rawTxn = await generateRawTransaction({ aptosConfig, sender, payload, options });

  if (feePayerAddress) {
    const signers: Array<AccountAddress> = secondarySignerAddresses
      ? secondarySignerAddresses.map((signer) => AccountAddress.fromHexInput({ input: signer }))
      : [];

    return {
      rawTransaction: rawTxn.bcsToBytes(),
      secondarySignerAddresses: signers,
      feePayerAddress: AccountAddress.fromHexInput({ input: feePayerAddress }),
    };
  }

  if (secondarySignerAddresses) {
    const signers: Array<AccountAddress> = secondarySignerAddresses.map((signer) =>
      AccountAddress.fromHexInput({ input: signer }),
    );

    return {
      rawTransaction: rawTxn.bcsToBytes(),
      secondarySignerAddresses: signers,
    };
  }
  // return the raw transaction
  return { rawTransaction: rawTxn.bcsToBytes() };
}

/**
 * Simluate a transaction before signing and submit to chain
 * @param args
 * @returns
 */
export function generateSignedTransactionForSimulation(args: SimulateTransactionData): Uint8Array {
  const { signerPublicKey, transaction, secondarySignersPublicKeys, feePayerPublicKey } = args;

  const deserializer = new Deserializer(transaction.rawTransaction);
  const desirializedTransaction = RawTransaction.deserialize(deserializer);

  // fee payer transaction
  if (transaction.feePayerAddress) {
    const transactionToSign = new FeePayerRawTransaction(
      desirializedTransaction,
      transaction.secondarySignerAddresses ?? [],
      transaction.feePayerAddress,
    );
    const accountAuthenticator = new AccountAuthenticatorEd25519(
      new Ed25519PublicKey({ hexInput: signerPublicKey.toUint8Array() }),
      new Ed25519Signature({ hexInput: new Uint8Array(64) }),
    );
    const secondaryAccountAuthenticators = secondarySignersPublicKeys!.map(
      (publicKey) =>
        new AccountAuthenticatorEd25519(
          new Ed25519PublicKey({ hexInput: publicKey.toUint8Array() }),
          new Ed25519Signature({ hexInput: new Uint8Array(64) }),
        ),
    );
    const feePayerAuthenticator = new AccountAuthenticatorEd25519(
      new Ed25519PublicKey({ hexInput: feePayerPublicKey!.toUint8Array() }),
      new Ed25519Signature({ hexInput: new Uint8Array(64) }),
    );
    const transactionAuthenticator = new TransactionAuthenticatorFeePayer(
      accountAuthenticator,
      transaction.secondarySignerAddresses ?? [],
      secondaryAccountAuthenticators ?? [],
      { address: transaction.feePayerAddress, authenticator: feePayerAuthenticator },
    );
    return new SignedTransaction(transactionToSign.raw_txn, transactionAuthenticator).bcsToBytes();
  }

  // multi agent transaction
  if (transaction.secondarySignerAddresses) {
    const transactionToSign = new MultiAgentRawTransaction(
      desirializedTransaction,
      transaction.secondarySignerAddresses,
    );

    const accountAuthenticator = new AccountAuthenticatorEd25519(
      new Ed25519PublicKey({ hexInput: signerPublicKey.toUint8Array() }),
      new Ed25519Signature({ hexInput: new Uint8Array(64) }),
    );

    const secondaryAccountAuthenticators = secondarySignersPublicKeys!.map(
      (publicKey) =>
        new AccountAuthenticatorEd25519(
          new Ed25519PublicKey({ hexInput: publicKey.toUint8Array() }),
          new Ed25519Signature({ hexInput: new Uint8Array(64) }),
        ),
    );

    const transactionAuthenticator = new TransactionAuthenticatorMultiAgent(
      accountAuthenticator,
      transaction.secondarySignerAddresses,
      secondaryAccountAuthenticators,
    );

    return new SignedTransaction(transactionToSign.raw_txn, transactionAuthenticator).bcsToBytes();
  }

  // raw transaction

  const accountAuthenticator = new AccountAuthenticatorEd25519(
    new Ed25519PublicKey({ hexInput: signerPublicKey.toUint8Array() }),
    new Ed25519Signature({ hexInput: new Uint8Array(64) }),
  );

  const transactionAuthenticator = new TransactionAuthenticatorEd25519(
    accountAuthenticator.public_key,
    accountAuthenticator.signature,
  );
  return new SignedTransaction(desirializedTransaction, transactionAuthenticator).bcsToBytes();
}

/**
 * Sign a transaction that can later be submitted to chain
 *
 * @param args.signer The signer account to sign the transaction
 * @param args.transaction A aptos transaction type to sign
 *
 * @return The signer AccountAuthenticator
 */
export function signTransaction(args: { signer: Account; transaction: AnyRawTransaction }): AccountAuthenticator {
  const { signer, transaction } = args;

  const transactionToSign = derieveTransactionType(transaction);

  // get the signing message
  const message = getSigningMessage(transactionToSign);

  // account.signMessage
  const signerSignature = signer.sign({ data: message });

  // return account authentication
  switch (signer.signingScheme) {
    case SigningScheme.Ed25519:
      return new AccountAuthenticatorEd25519(
        new Ed25519PublicKey({ hexInput: signer.publicKey.toUint8Array() }),
        new Ed25519Signature({ hexInput: signerSignature.toUint8Array() }),
      );
    case SigningScheme.Secp256k1Ecdsa:
      return new AccountAuthenticatorSecp256k1(
        new Secp256k1PublicKey({ hexInput: signer.publicKey.toUint8Array() }),
        new Secp256k1Signature({ hexInput: signerSignature.toUint8Array() }),
      );
    // TODO support MultiEd25519
    default:
      throw new Error(`can't sign transaction, signing scheme not supported ${signer.signingScheme}`);
  }
}

/**
 * Prepare a transaction to be submitted to chain
 *
 * @param args.transaction A aptos transaction type
 * @param args.senderAuthenticator The account authenticator of the transaction sender
 * @param args.secondarySignerAuthenticators optional. For when the transaction is a multi signers transaction
 *
 * @returns A SignedTransaction
 */
export function generateSignedTransaction(args: {
  transaction: AnyRawTransaction;
  senderAuthenticator: AccountAuthenticator;
  secondarySignerAuthenticators?: {
    feePayerAuthenticator?: AccountAuthenticator;
    additionalSignersAuthenticators?: Array<AccountAuthenticator>;
  };
}): Uint8Array {
  const { transaction, senderAuthenticator, secondarySignerAuthenticators } = args;

  const transactionToSubmit = derieveTransactionType(transaction);

  if (secondarySignerAuthenticators) {
    return generateMultiSignersSignedTransaction(
      transactionToSubmit as MultiAgentRawTransaction | FeePayerRawTransaction,
      senderAuthenticator,
      secondarySignerAuthenticators,
    );
  }

  // submit single signer transaction

  // deserialize the senderAuthenticator
  const deserializer = new Deserializer(senderAuthenticator.bcsToBytes());
  const accountAuthenticator = AccountAuthenticator.deserialize(deserializer);
  // check what instance is accountAuthenticator
  if (accountAuthenticator instanceof AccountAuthenticatorEd25519) {
    const transactionAuthenticator = new TransactionAuthenticatorEd25519(
      accountAuthenticator.public_key,
      accountAuthenticator.signature,
    );
    // return signed transaction
    return new SignedTransaction(transactionToSubmit as RawTransaction, transactionAuthenticator).bcsToBytes();
  }

  if (accountAuthenticator instanceof AccountAuthenticatorSecp256k1) {
    const transactionAuthenticator = new TransactionAuthenticatorSecp256k1(
      accountAuthenticator.public_key,
      accountAuthenticator.signature,
    );
    // return signed transaction
    return new SignedTransaction(transactionToSubmit as RawTransaction, transactionAuthenticator).bcsToBytes();
  }

  throw new Error(
    `can not generate signed transaction, ${accountAuthenticator} is not a supported account authentication`,
  );
}

/**
 * Derive the raw transaction type - FeePayerRawTransaction or MultiAgentRawTransaction or RawTransaction
 *
 * @param transaction A aptos transaction type
 *
 * @returns FeePayerRawTransaction | MultiAgentRawTransaction | RawTransaction
 */
export function derieveTransactionType(transaction: AnyRawTransaction): AnyRawTransactionInstance {
  const deserializer = new Deserializer(transaction.rawTransaction);
  const desirializedTransaction = RawTransaction.deserialize(deserializer);

  if (transaction.feePayerAddress) {
    return new FeePayerRawTransaction(
      desirializedTransaction,
      transaction.secondarySignerAddresses ?? [],
      transaction.feePayerAddress,
    );
  }
  if (transaction.secondarySignerAddresses) {
    return new MultiAgentRawTransaction(desirializedTransaction, transaction.secondarySignerAddresses);
  }

  return desirializedTransaction as RawTransaction;
}

/**
 * Generate a multi signers signed transaction that can be submitted to chain
 *
 * @param transaction MultiAgentRawTransaction | FeePayerRawTransaction
 * @param senderAuthenticator The account authenticator of the transaction sender
 * @param secondarySignerAuthenticators The extra signers account Authenticators
 *
 * @returns A SignedTransaction
 */
export function generateMultiSignersSignedTransaction(
  transaction: MultiAgentRawTransaction | FeePayerRawTransaction,
  senderAuthenticator: AccountAuthenticator,
  secondarySignerAuthenticators: {
    feePayerAuthenticator?: AccountAuthenticator;
    additionalSignersAuthenticators?: Array<AccountAuthenticator>;
  },
) {
  if (transaction instanceof FeePayerRawTransaction) {
    if (!secondarySignerAuthenticators.feePayerAuthenticator) {
      throw new Error("Must provide a feePayerAuthenticator argument to generate a signed fee payer transaction");
    }
    const { feePayerAuthenticator, additionalSignersAuthenticators } = secondarySignerAuthenticators;
    const txAuthenticatorFeePayer = new TransactionAuthenticatorFeePayer(
      senderAuthenticator,
      transaction.secondary_signer_addresses,
      additionalSignersAuthenticators ?? [],
      { address: transaction.fee_payer_address, authenticator: feePayerAuthenticator },
    );
    return new SignedTransaction(transaction.raw_txn, txAuthenticatorFeePayer).bcsToBytes();
  }
  if (transaction instanceof MultiAgentRawTransaction) {
    if (!secondarySignerAuthenticators.additionalSignersAuthenticators) {
      throw new Error(
        "Must provide a additionalSignersAuthenticators argument to generate a signed multi agent transaction",
      );
    }
    const { additionalSignersAuthenticators } = secondarySignerAuthenticators;
    const multiAgentAuthenticator = new TransactionAuthenticatorMultiAgent(
      senderAuthenticator,
      transaction.secondary_signer_addresses,
      additionalSignersAuthenticators ?? [],
    );
    return new SignedTransaction(transaction.raw_txn, multiAgentAuthenticator).bcsToBytes();
  }

  throw new Error(
    `cann not prepare multi signers transaction to submission, ${typeof transaction} transaction is not supported`,
  );
}

const RAW_TRANSACTION_SALT = "APTOS::RawTransaction";
const RAW_TRANSACTION_WITH_DATA_SALT = "APTOS::RawTransactionWithData";

export function getSigningMessage(rawTxn: AnyRawTransactionInstance): Uint8Array {
  const hash = sha3Hash.create();

  if (rawTxn instanceof RawTransaction) {
    hash.update(RAW_TRANSACTION_SALT);
  } else if (rawTxn instanceof MultiAgentRawTransaction) {
    hash.update(RAW_TRANSACTION_WITH_DATA_SALT);
  } else if (rawTxn instanceof FeePayerRawTransaction) {
    hash.update(RAW_TRANSACTION_WITH_DATA_SALT);
  } else {
    throw new Error(`Unknown transaction type to sign on: ${rawTxn}`);
  }

  const prefix = hash.digest();

  const body = rawTxn.bcsToBytes();

  const mergedArray = new Uint8Array(prefix.length + body.length);
  mergedArray.set(prefix);
  mergedArray.set(body, prefix.length);

  return mergedArray;
}
