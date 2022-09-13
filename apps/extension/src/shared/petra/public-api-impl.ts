// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount,
  AptosClient,
  HexString,
  Types,
} from 'aptos';
import { DappErrorType, makeTransactionError } from 'core/types/errors';
import { PublicAccount } from 'core/types/stateTypes';
import Permissions from 'core/utils/permissions';
import { triggerDisconnect } from 'core/utils/providerEvents';
import { PersistentStorage, SessionStorage } from 'shared/storage';
import { defaultCustomNetworks, defaultNetworkName, defaultNetworks } from 'shared/types';
import {
  DappInfo,
  PermissionHandler,
  SignAndSubmitTransactionPermissionApproval,
  SignTransactionPermissionApproval,
} from 'shared/permissions';
import { PetraPublicApi, SignMessagePayload } from './public-api';

// region Utils

/**
 * Get the active public account from persistent storage
 */
async function getActiveAccount() {
  const { activeAccountAddress, activeAccountPublicKey } = await PersistentStorage.get([
    'activeAccountAddress',
    'activeAccountPublicKey',
  ]);
  return activeAccountAddress !== undefined && activeAccountPublicKey !== undefined
    ? {
      address: activeAccountAddress,
      publicKey: activeAccountPublicKey,
    } as PublicAccount
    : undefined;
}

/**
 * Get the active network from persistent storage
 */
async function getActiveNetwork() {
  const { activeNetworkName, customNetworks } = await PersistentStorage.get([
    'activeNetworkName',
    'customNetworks',
  ]);

  const networks = { ...defaultNetworks, ...(customNetworks ?? defaultCustomNetworks) };
  return networks[activeNetworkName ?? defaultNetworkName];
}

/**
 * Return the active account, or throw if not connected to dapp
 * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
 */
async function ensureAccountConnected(domain: string) {
  const account = await getActiveAccount();
  const isAllowed = account !== undefined
    && await Permissions.isDomainAllowed(domain, account.address);
  if (!isAllowed) {
    throw DappErrorType.UNAUTHORIZED;
  }
  return account;
}

/**
 * Get signer account from address
 * @param address
 */
async function getAptosAccount(address: string) {
  const { accounts } = await SessionStorage.get(['accounts']);
  if (accounts === undefined) {
    throw new Error('accounts are locked');
  }
  const { privateKey } = accounts[address];
  return new AptosAccount(
    HexString.ensure(privateKey).toUint8Array(),
    address,
  );
}

/**
 * Create and sign a transaction from a payload
 * @param client
 * @param signerAddress
 * @param payload
 * @param maxGasFee
 */
async function signTransaction(
  client: AptosClient,
  signerAddress: string,
  payload: Types.EntryFunctionPayload,
  maxGasFee?: number,
) {
  const signer = await getAptosAccount(signerAddress);
  const txn = await client.generateTransaction(signerAddress, payload, {
    max_gas_amount: maxGasFee !== undefined ? `${maxGasFee}` : undefined,
  });
  return client.signTransaction(signer, txn);
}

// endregion

export const PetraPublicApiImpl = {

  /**
   * Get the active public account
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   */
  async account({ domain }: DappInfo) {
    return ensureAccountConnected(domain);
  },

  /**
   * Request the user to connect the active account to the dapp
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   * @throws {DappErrorType.USER_REJECTION} when user rejects prompt
   * @throws {DappErrorType.TIME_OUT} when prompt times out
   */
  async connect(dappInfo: DappInfo) {
    const account = await getActiveAccount();

    const connectRequest = PermissionHandler.requestPermission(
      dappInfo,
      { type: 'connect' },
    );

    // Check for backward compatibility, ideally should be removed
    if (account === undefined) {
      throw DappErrorType.NO_ACCOUNTS;
    }

    // TODO: should get account from here
    await connectRequest;
    await Permissions.addDomain(dappInfo.domain, account!.address);
    return account;
  },

  /**
   * Disconnect the active account from the dapp
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   */
  async disconnect({ domain }: DappInfo) {
    const { address } = await ensureAccountConnected(domain);
    triggerDisconnect();
    await Permissions.removeDomain(domain, address);
  },

  /**
   * Check if the active account is connected to the dapp
   */
  async isConnected({ domain }: DappInfo) {
    const account = await getActiveAccount();
    return account !== undefined
      && Permissions.isDomainAllowed(domain, account.address);
  },

  /**
   * Get the active network name
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   */
  async network({ domain }: DappInfo) {
    await ensureAccountConnected(domain);
    const { name } = await getActiveNetwork();
    return name;
  },

  /**
   * Create and submit a signed transaction from a payload
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   * @throws {DappErrorType.USER_REJECTION} if the request was rejected
   * @throws {DappErrorType.TIME_OUT} if the request timed out
   * @throws {DappError} if the transaction fails
   */
  async signAndSubmitTransaction(dappInfo: DappInfo, payload: Types.EntryFunctionPayload) {
    const { address } = await ensureAccountConnected(dappInfo.domain);

    const { maxGasFee } = await PermissionHandler.requestPermission(
      dappInfo,
      { payload, type: 'signAndSubmitTransaction' },
    ) as SignAndSubmitTransactionPermissionApproval;

    // handle rejection and timeout

    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    try {
      const signedTxn = await signTransaction(
        aptosClient,
        address,
        payload,
        maxGasFee,
      );
      return await aptosClient.submitTransaction(signedTxn);
    } catch (err) {
      // Trace original error without rethrowing (this is a dapp error)
      // eslint-disable-next-line no-console
      console.trace(err);
      throw makeTransactionError(err);
    }
  },

  async signMessage(dappInfo: DappInfo, {
    address = false,
    application = false,
    chainId = false,
    message,
    nonce,
  }: SignMessagePayload) {
    const { address: accountAddress } = await ensureAccountConnected(dappInfo.domain);

    await PermissionHandler.requestPermission(
      dappInfo,
      { message, type: 'signMessage' },
    );

    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    const clientChainId = await aptosClient.getChainId();

    const signer = await getAptosAccount(accountAddress);
    const encoder = new TextEncoder();
    const prefix = 'APTOS';
    let messageToBeSigned = prefix;

    if (address) {
      messageToBeSigned += `\naddress: ${accountAddress}`;
    }

    if (application) {
      messageToBeSigned += `\napplication: ${dappInfo.domain}`;
    }

    if (chainId) {
      messageToBeSigned += `\nchainId: ${clientChainId}`;
    }

    messageToBeSigned += `\nmessage: ${message}`;
    messageToBeSigned += `\nnonce: ${nonce}`;

    const messageBytes = encoder.encode(messageToBeSigned);
    const signature = signer.signBuffer(messageBytes);
    const signatureString = signature.noPrefix();
    return {
      address: accountAddress,
      application: dappInfo.domain,
      chainId: clientChainId,
      fullMessage: messageToBeSigned,
      message,
      nonce,
      prefix,
      signature: signatureString,
    };
  },

  /**
   * Create a signed transaction from a payload
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   * @throws {DappErrorType.USER_REJECTION} if the request was rejected
   * @throws {DappErrorType.TIME_OUT} if the request timed out
   * @throws {DappError} if the transaction fails
   */
  async signTransaction(dappInfo: DappInfo, payload: Types.EntryFunctionPayload) {
    const { address } = await ensureAccountConnected(dappInfo.domain);
    const { maxGasFee } = await PermissionHandler.requestPermission(
      dappInfo,
      { payload, type: 'signTransaction' },
    ) as SignTransactionPermissionApproval;

    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    try {
      return await signTransaction(aptosClient, address, payload, maxGasFee);
    } catch (err) {
      // Trace original error without rethrowing (this is a dapp error)
      // eslint-disable-next-line no-console
      console.trace(err);
      throw makeTransactionError(err);
    }
  },
};

export type PetraPublicApiMethod = keyof PetraPublicApi;
export function isAllowedMethodName(method: string): method is PetraPublicApiMethod {
  return Object.keys(PetraPublicApiImpl).includes(method);
}

export default PetraPublicApiImpl;
