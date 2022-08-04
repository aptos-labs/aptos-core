// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, BCS } from 'aptos';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import { sign } from 'tweetnacl';
import { MessageMethod, PermissionType } from '../core/types/dappTypes';
import { getBackgroundAptosAccountState, getBackgroundNodeUrl } from '../core/utils/account';
import Permissions from '../core/utils/permissions';
import { DappErrorType } from '../core/types/errors';

// Utils

async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const url = new URL(tabs[0].url);
  return url.hostname;
}

async function signTransaction(client, account, transaction) {
  const address = account.address().hex();
  const txn = await client.generateTransaction(address, transaction);
  return client.signTransaction(account, txn);
}

async function checkConnected(account, sendResponse) {
  if (Permissions.isDomainAllowed(await getCurrentDomain(), account.address().hex())) {
    return true;
  }
  sendResponse({ error: DappErrorType.UNAUTHORIZED });
  return false;
}

function rejectRequest(sendResponse) {
  sendResponse({ error: DappErrorType.USER_REJECTION });
}

// Aptos dApp methods

function getAccountAddress(account, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
    return;
  }

  if (account.address()) {
    sendResponse({ address: account.address().hex(), publicKey: account.pubKey().hex() });
  } else {
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
  }
}

async function connect(account, sendResponse) {
  if (await Permissions.requestPermissions(
    PermissionType.CONNECT,
    await getCurrentDomain(),
    account.address().hex(),
  )) {
    getAccountAddress(account, sendResponse);
  } else {
    rejectRequest(sendResponse);
  }
}

async function disconnect(account, sendResponse) {
  await Permissions.removeDomain(await getCurrentDomain(), account.address().hex());
  sendResponse({});
}

async function isConnected(account, sendResponse) {
  const status = await Permissions.isDomainAllowed(
    await getCurrentDomain(),
    account.address().hex(),
  );
  sendResponse(status);
}

async function signAndSubmitTransaction(client, account, transaction, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    PermissionType.SIGN_AND_SUBMIT_TRANSACTION,
    await getCurrentDomain(),
    account.address().hex(),
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }
  try {
    const signedTransaction = await signTransaction(client, account, transaction);
    const response = await client.submitTransaction(signedTransaction);
    sendResponse(response);
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function signTransactionAndSendResponse(client, account, transaction, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    PermissionType.SIGN_TRANSACTION,
    await getCurrentDomain(),
    account.address().hex(),
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }
  try {
    const signedTransaction = await signTransaction(client, account, transaction);
    sendResponse({ signedTransaction });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function signMessage(account, message, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    PermissionType.SIGN_MESSAGE,
    await getCurrentDomain(),
    account.address().hex(),
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }
  try {
    const serializer = new BCS.Serializer();
    serializer.serializeStr(message);
    const signature = sign(serializer.getBytes(), account.signingKey.secretKey);
    const signedMessage = Buffer.from(signature).toString('hex');
    sendResponse({ signedMessage });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function handleDappRequest(request, sendResponse) {
  const account = await getBackgroundAptosAccountState();
  const network = await getBackgroundNodeUrl();
  if (account === undefined) {
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
    return;
  }

  // The fetch adapter is neccessary to use axios from a service worker
  const client = new AptosClient(network, { adapter: fetchAdapter });
  switch (request.method) {
    case MessageMethod.CONNECT:
      connect(account, sendResponse);
      break;
    case MessageMethod.DISCONNECT:
      disconnect(account, sendResponse);
      break;
    case MessageMethod.IS_CONNECTED:
      isConnected(account, sendResponse);
      break;
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      getAccountAddress(account, sendResponse);
      break;
    case MessageMethod.SIGN_AND_SUBMIT_TRANSACTION:
      signAndSubmitTransaction(client, account, request.args.transaction, sendResponse);
      break;
    case MessageMethod.SIGN_TRANSACTION:
      signTransactionAndSendResponse(client, account, request.args.transaction, sendResponse);
      break;
    case MessageMethod.SIGN_MESSAGE:
      signMessage(account, request.args.message, sendResponse);
      break;
    default:
      // method not supported
      break;
  }
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  try {
    handleDappRequest(request, sendResponse);
  } catch (error) {
    sendResponse({ error });
  }
  return true;
});
