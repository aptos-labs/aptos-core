// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import {
  AptosAccount,
  AptosClient,
  HexString,
  Types,
} from 'aptos';
import axios from 'axios';
import { Permission, warningPrompt } from 'core/types/dappTypes';
import { DappErrorType, makeTransactionError } from 'core/types/errors';
import { PublicAccount } from 'core/types/stateTypes';
import Permissions from 'core/utils/permissions';
import PromptPresenter from 'core/utils/promptPresenter';
import { PersistentStorage, SessionStorage } from 'shared/storage';
import { defaultCustomNetworks, defaultNetworkName, defaultNetworks } from 'shared/types';
import { PetraPublicApi, SignMessagePayload } from './public-api';

// The fetch adapter is necessary to use axios from a service worker
// TODO: maybe move this under background.ts
axios.defaults.adapter = fetchAdapter;

// region Utils

/**
 * Get the domain of the active tab of the current window
 */
async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tabs[0]?.url === undefined) {
    throw new Error("Couldn't retrieve tab URL");
  }
  const url = new URL(tabs[0].url);
  return url.hostname;
}

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
 * Return the active account, or throw if not available
 * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
 */
async function ensureActiveAccount(promptIfNoAccount = false) {
  const activeAccount = await getActiveAccount();
  if (activeAccount === undefined) {
    if (promptIfNoAccount) {
      await PromptPresenter.promptUser(warningPrompt());
    }
    throw DappErrorType.NO_ACCOUNTS;
  }
  return activeAccount;
}

/**
 * Return the active account, or throw if not connected to dapp
 * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
 * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
 */
async function ensureAccountConnected() {
  const account = await ensureActiveAccount();
  const domain = await getCurrentDomain();
  const isAllowed = await Permissions.isDomainAllowed(domain, account.address);
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
 */
async function signTransaction(
  client: AptosClient,
  signerAddress: string,
  payload: Types.EntryFunctionPayload,
) {
  const signer = await getAptosAccount(signerAddress);
  const txn = await client.generateTransaction(signerAddress, payload);
  return client.signTransaction(signer, txn);
}

// endregion

export const PetraPublicApiImpl: PetraPublicApi = {

  /**
   * Get the active public account
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   */
  async account() {
    return ensureAccountConnected();
  },

  /**
   * Request the user to connect the active account to the dapp
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   */
  async connect() {
    const activeAccount = await ensureActiveAccount(true);
    const domain = await getCurrentDomain();
    const allowed = await Permissions.requestPermissions(
      Permission.CONNECT,
      domain,
      activeAccount.address,
    );

    if (!allowed) {
      throw DappErrorType.USER_REJECTION;
    }

    return activeAccount;
  },

  /**
   * Disconnect the active account from the dapp
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   */
  async disconnect() {
    const { address } = await ensureAccountConnected();
    const domain = await getCurrentDomain();
    await Permissions.removeDomain(domain, address);
  },

  /**
   * Check if the active account is connected to the dapp
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   */
  async isConnected() {
    const { address } = await ensureActiveAccount();
    const domain = await getCurrentDomain();
    return Permissions.isDomainAllowed(domain, address);
  },

  /**
   * Get the active network name
   */
  async network() {
    await ensureAccountConnected();
    const { name } = await getActiveNetwork();
    return name;
  },

  /**
   * Create and submit a signed transaction from a payload
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   * @throws {DappErrorType.USER_REJECTION} if the request was rejected
   * @throws {DappError} if the transaction fails
   */
  async signAndSubmitTransaction(payload: Types.EntryFunctionPayload) {
    const { address } = await ensureAccountConnected();
    const domain = await getCurrentDomain();
    const permission = await Permissions.requestPermissions(
      Permission.SIGN_AND_SUBMIT_TRANSACTION,
      domain,
      address,
    );

    if (!permission) {
      throw DappErrorType.USER_REJECTION;
    }

    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    try {
      const signedTxn = await signTransaction(aptosClient, address, payload);
      return await aptosClient.submitTransaction(signedTxn);
    } catch (err) {
      // Trace original error without rethrowing (this is a dapp error)
      // eslint-disable-next-line no-console
      console.trace(err);
      throw makeTransactionError(err);
    }
  },

  async signMessage({
    address = false,
    application = false,
    chainId = false,
    message,
    nonce,
  }: SignMessagePayload) {
    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    const clientChainId = await aptosClient.getChainId();
    const { address: accountAddress } = await ensureAccountConnected();
    const domain = await getCurrentDomain();
    const permission = await Permissions.requestPermissions(
      Permission.SIGN_MESSAGE,
      domain,
      accountAddress,
    );

    if (!permission) {
      throw DappErrorType.USER_REJECTION;
    }

    const signer = await getAptosAccount(accountAddress);
    const encoder = new TextEncoder();
    const prefix = 'APTOS';
    let messageToBeSigned = prefix;

    if (address) {
      messageToBeSigned += `\naddress: ${accountAddress}`;
    }

    if (application) {
      messageToBeSigned += `\napplication: ${domain}`;
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
      application: domain,
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
   * @throws {DappErrorType.NO_ACCOUNTS} if no active account is available
   * @throws {DappErrorType.UNAUTHORIZED} if the active account is not connected to the dapp
   * @throws {DappErrorType.USER_REJECTION} if the request was rejected
   * @throws {DappError} if the transaction fails
   */
  async signTransaction(payload: Types.EntryFunctionPayload) {
    const { address } = await ensureAccountConnected();
    const domain = await getCurrentDomain();
    const allowed = await Permissions.requestPermissions(
      Permission.SIGN_TRANSACTION,
      domain,
      address,
    );

    if (!allowed) {
      throw DappErrorType.USER_REJECTION;
    }

    const { nodeUrl } = await getActiveNetwork();
    const aptosClient = new AptosClient(nodeUrl);
    try {
      return await signTransaction(aptosClient, address, payload);
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
