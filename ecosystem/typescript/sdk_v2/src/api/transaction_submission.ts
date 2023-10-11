import { AptosConfig } from "./aptos_config";
import { Account } from "../core/account";
import { AccountAuthenticator } from "../transactions/authenticator/account";
import { MimeType, postAptosFullNode } from "../client";
import {
  AnyRawTransaction,
  GenerateMultiAgentRawTransactionInput,
  MultiAgentTransaction,
  GenerateTransactionInput,
  FeePayerTransaction,
  GenerateFeePayerRawTransactionInput,
  GenerateSingleSignerRawTransactionInput,
  SingleSignerTransaction,
  SimulateTransactionData,
} from "../transactions/types";
import { UserTransactionResponse, PendingTransactionResponse } from "../types";
import {
  generateSignedTransaction,
  generateSignedTransactionForSimulation,
  generateTransaction,
  generateTransactionPayload,
  signTransaction,
} from "../transactions/transaction_builder/transaction_builder";

export class TransactionSubmission {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * We are defining function signatures, each with its specific input and output.
   * These are the possible function signature for `generateTransaction` function.
   * When we call `generateTransaction` function with the relevant type properties,
   * Typescript can infer the return type based on the appropriate function overload.
   */
  async generateTransaction(args: GenerateSingleSignerRawTransactionInput): Promise<SingleSignerTransaction>;
  async generateTransaction(args: GenerateFeePayerRawTransactionInput): Promise<FeePayerTransaction>;
  async generateTransaction(args: GenerateMultiAgentRawTransactionInput): Promise<MultiAgentTransaction>;
  async generateTransaction(args: GenerateTransactionInput): Promise<AnyRawTransaction>;

  /**
   * Generates any transaction by passing in the required arguments
   *
   * @param args.sender The transaction sender's account address as a HexInput
   * @param args.data EntryFunctionData | ScriptData | MultiSigData
   * @param feePayerAddress optional. For a fee payer (aka sponsored) transaction
   * @param secondarySignerAddresses optional. For a multi agent or fee payer (aka sponsored) transactions
   * @param args.options optional. GenerateTransactionOptions type
   *
   * @example
   * For a singale signer entry function
   * move function name, move function type arguments, move function arguments
   * `
   * data: {
   *  function:"0x1::aptos_account::transfer",
   *  type_arguments:[]
   *  arguments:[recieverAddress,10]
   * }
   * `
   *
   * @example
   * For a singale signer script function
   * module bytecode, move function type arguments, move function arguments
   * ```
   * data: {
   *  bytecode:"0x001234567",
   *  type_arguments:[],
   *  arguments:[recieverAddress,10]
   * }
   * ```
   *
   * @return A raw transaction type (note that it holds the raw transaction as a bcs serialized data)
   * ```
   * {
   *  rawTransaction: Uint8Array,
   *  secondarySignerAddresses? : Array<AccountAddress>,
   *  feePayerAddress?: AccountAddress
   * }
   * ```
   */
  async generateTransaction(args: GenerateTransactionInput): Promise<AnyRawTransaction> {
    const { sender, data, options, secondarySignerAddresses, feePayerAddress } = args;
    const payload = await generateTransactionPayload(data);
    const rawTransaction = await generateTransaction({
      aptosConfig: this.config,
      sender,
      payload,
      options,
      secondarySignerAddresses,
      feePayerAddress,
    });
    return rawTransaction;
  }

  /**
   * Sign a transaction that can later be submitted to chain
   *
   * @param args.signer The signer account to sign the transaction
   * @param args.transaction A raw transaction type (note that it holds the raw transaction as a bcs serialized data)
   * ```
   * {
   *  rawTransaction: Uint8Array,
   *  secondarySignerAddresses? : Array<AccountAddress>,
   *  feePayerAddress?: AccountAddress
   * }
   * ```
   *
   * @return The signer AccountAuthenticator
   */
  /* eslint-disable class-methods-use-this */
  signTransaction(args: { signer: Account; transaction: AnyRawTransaction }): AccountAuthenticator {
    const accountAuthenticator = signTransaction({ ...args });
    return accountAuthenticator;
  }

  /**
   * Simulates a transaction before singing it.
   *
   * @param signerPublicKey The signer pubic key
   * @param transaction The raw transaction to simulate
   * @param secondarySignersPublicKeys optional. For when the transaction is a multi signers transaction
   * @param feePayerPublicKey optional. For when the transaction is a fee payer (aka sponsored) transaction
   * @param options optional. A config to simulate the transaction with
   */
  async simulateTransaction(args: SimulateTransactionData): Promise<Array<UserTransactionResponse>> {
    const signedTransaction = generateSignedTransactionForSimulation({ ...args });
    const { data } = await postAptosFullNode<Uint8Array, Array<UserTransactionResponse>>({
      aptosConfig: this.config,
      body: signedTransaction,
      path: "transactions/simulate",
      params: {
        estimate_gas_unit_price: args.options?.estimateGasUnitPrice ?? false,
        estimate_max_gas_amount: args.options?.estimateMaxGasAmount ?? false,
        estimate_prioritized_gas_unit_price: args.options?.estimatePrioritizedGasUnitPrice ?? false,
      },
      originMethod: "simulateTransaction",
      contentType: MimeType.BCS_SIGNED_TRANSACTION,
    });
    return data;
  }

  /**
   * Submit transaction to chain
   *
   * @param args.transaction A aptos transaction type
   * @param args.senderAuthenticator The account authenticator of the transaction sender
   * @param args.secondarySignerAuthenticators optional. For when the transaction is a multi signers transaction
   *
   * @return PendingTransactionResponse
   */
  async submitTransaction(args: {
    transaction: AnyRawTransaction;
    senderAuthenticator: AccountAuthenticator;
    secondarySignerAuthenticators?: {
      feePayerAuthenticator?: AccountAuthenticator;
      additionalSignersAuthenticators?: Array<AccountAuthenticator>;
    };
  }): Promise<PendingTransactionResponse> {
    const signedTransaction = generateSignedTransaction({ ...args });
    const { data } = await postAptosFullNode<Uint8Array, PendingTransactionResponse>({
      aptosConfig: this.config,
      body: signedTransaction,
      path: "transactions",
      originMethod: "submitTransaction",
      contentType: MimeType.BCS_SIGNED_TRANSACTION,
    });
    return data;
  }
}
