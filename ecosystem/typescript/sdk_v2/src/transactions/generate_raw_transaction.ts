import { AccountAddress, AptosAccount } from "../account";
import { AptosConfig } from "../api/aptos_config";
import {
  AptosEntryFunctionTransactionPayload,
  TransactionOptions,
  RawTransaction,
  MultiAgentRawTransaction,
  FeePayerRawTransaction,
  AptosMultiAgentTransactionPayload,
  AptosFeePayerTransactionPayload,
  AptosScriptTransactionPayload,
  Script,
  TransactionPayloadScript,
  ChainId,
  TransactionPayload,
  AptosMultiSigTransactionPayload,
} from "./types";
import { DEFAULT_MAX_GAS_AMOUNT, DEFAULT_TXN_EXP_SEC_FROM_NOW, MaybeHexString } from "../utils";
import { RemoteABIBuilderConfig, TransactionBuilderRemoteABI } from "./builder";
import { AuthenticationKey, Ed25519PublicKey, MultiEd25519PublicKey } from "../crypto";
import { getData } from "../internal/account";
import { getChainId } from "../internal/general";
import { estimateGasPrice } from "../internal/transaction";

/** Generates an entry function transaction request that can be submitted to produce a raw transaction that
 * can be signed, which upon being signed can be submitted to the blockchain
 * This function fetches the remote ABI and uses it to serialized the data, therefore
 * users don't need to handle serialization by themselves.
 * @param sender Hex-encoded 32 byte Aptos account address of transaction sender
 * @param payload Entry function transaction payload type
 * @param options Options allow to overwrite default transaction options.
 * @returns A raw transaction object
 */
export async function generateEntryFunctionRawTransaction(
  sender: MaybeHexString,
  payload: AptosEntryFunctionTransactionPayload,
  config: AptosConfig,
  options?: TransactionOptions,
): Promise<RawTransaction> {
  const rawTxn = await generateRawTransactionFromRemoteABI(sender, payload, config, options);
  return rawTxn;
}

export async function generateScriptRawTransaction(
  sender: MaybeHexString,
  payload: AptosScriptTransactionPayload,
  config: AptosConfig,
  options?: TransactionOptions,
): Promise<RawTransaction> {
  const scriptPayload = new TransactionPayloadScript(
    new Script(payload.bytecode, payload.type_arguments, payload.arguments),
  );

  return generateRawTransaction(sender, scriptPayload, config, options);
}

export async function generateMultiAgentRawTransaction(
  sender: MaybeHexString,
  payload: AptosMultiAgentTransactionPayload,
  config: AptosConfig,
  options?: TransactionOptions,
): Promise<MultiAgentRawTransaction> {
  const rawTxn = await generateRawTransactionFromRemoteABI(sender, payload, config, options);

  const signers: Array<AccountAddress> = payload.secondary_signer_addresses.map((account) =>
    AccountAddress.fromHex(account.address()),
  );

  const multiAgentTxn = new MultiAgentRawTransaction(rawTxn, signers);
  return multiAgentTxn;
}

export async function generateFeePayerRawTransaction(
  sender: MaybeHexString,
  payload: AptosFeePayerTransactionPayload,
  config: AptosConfig,
  options?: TransactionOptions,
): Promise<FeePayerRawTransaction> {
  const rawTxn = await generateRawTransactionFromRemoteABI(sender, payload, config, options);

  const receivers: Array<AccountAddress> = payload.secondary_signer_addresses.map((account) =>
    AccountAddress.fromHex(account.address()),
  );

  const feePayerTxn = new FeePayerRawTransaction(
    rawTxn,
    receivers,
    AccountAddress.fromHex(payload.fee_payer.address()),
  );
  return feePayerTxn;
}

export async function generateMultiSigRawTransaction(
  mutisigAccountAddress: MaybeHexString,
  payload: AptosMultiSigTransactionPayload,
  config: AptosConfig,
  options?: TransactionOptions,
): Promise<RawTransaction> {
  const rawTxn = await generateRawTransactionFromRemoteABI(mutisigAccountAddress, payload, config, options);

  return rawTxn;
}

export function createMultisigAccount(multisig_addresses: Array<AptosAccount>, threshold: number): string {
  const signers: Array<Ed25519PublicKey> = multisig_addresses.map(
    (account) => new Ed25519PublicKey(account.signingKey.publicKey),
  );

  const multiSigPublicKey = new MultiEd25519PublicKey(signers, threshold);

  const authKey = AuthenticationKey.fromMultiEd25519PublicKey(multiSigPublicKey);
  const mutisigAccountAddress = authKey.derivedAddress().hex();
  return mutisigAccountAddress;
}

async function generateRawTransactionFromRemoteABI(
  sender: MaybeHexString,
  payload: AptosEntryFunctionTransactionPayload,
  aptosConfig: AptosConfig,
  options?: TransactionOptions,
): Promise<RawTransaction> {
  const config: RemoteABIBuilderConfig = { sender };
  if (options?.sequenceNumber) {
    config.sequenceNumber = options.sequenceNumber;
  }

  if (options?.gasUnitPrice) {
    config.gasUnitPrice = options.gasUnitPrice;
  }

  if (options?.maxGasAmount) {
    config.maxGasAmount = options.maxGasAmount;
  }

  if (options?.expirationTimestampSeconds) {
    const timestamp = Number.parseInt(options.expirationTimestampSeconds, 10);
    config.expSecFromNow = timestamp - Math.floor(Date.now() / 1000);
  }
  const builder = new TransactionBuilderRemoteABI(aptosConfig, config);
  const rawTxn = await builder.build(payload.function, payload.type_arguments, payload.arguments);
  return rawTxn;
}

async function generateRawTransaction(
  sender: MaybeHexString,
  payload: TransactionPayload,
  aptosConfig: AptosConfig,
  options?: TransactionOptions,
): Promise<RawTransaction> {
  const [{ sequence_number: sequenceNumber }, chainId, { gas_estimate: gasEstimate }] = await Promise.all([
    options?.sequenceNumber
      ? Promise.resolve({ sequence_number: options.sequenceNumber })
      : getData(aptosConfig, sender),
    options?.chainId ? Promise.resolve(options.chainId) : getChainId(aptosConfig),
    options?.gasUnitPrice ? Promise.resolve({ gas_estimate: options.gasUnitPrice }) : estimateGasPrice(aptosConfig),
  ]);

  const { maxGasAmount, gasUnitPrice, expireTimestamp } = {
    maxGasAmount: BigInt(DEFAULT_MAX_GAS_AMOUNT),
    gasUnitPrice: BigInt(gasEstimate),
    expireTimestamp: BigInt(Math.floor(Date.now() / 1000) + DEFAULT_TXN_EXP_SEC_FROM_NOW),
    ...options,
  };

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(sender),
    BigInt(sequenceNumber),
    payload,
    maxGasAmount,
    gasUnitPrice,
    expireTimestamp,
    new ChainId(chainId),
  );

  return rawTxn;
}
