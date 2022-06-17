// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from 'aptos';
import { DEVNET_NODE_URL } from '../core/constants';
import { MessageMethod } from '../core/types';
import { getBackgroundAptosAccountState } from '../core/utils/account';
import Permissions from '../core/utils/permissions';

// Utils

async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const url = new URL(tabs[0].url);
  return url.hostname;
}

async function signTransaction(client, account, transaction) {
  const address = account.address();
  const txn = await client.generateTransaction(address, transaction);
  return client.signTransaction(account, txn);
}

async function checkConnected(sendResponse) {
  if (Permissions.isDomainAllowed(await getCurrentDomain())) {
    return true;
  }
  sendResponse({ error: 'App not connected - call aptos.connect()' });
  return false;
}

// Aptos dApp methods

function getAccountAddress(account, sendResponse) {
  if (!checkConnected(sendResponse)) {
    return;
  }

  if (account.address()) {
    sendResponse({ address: account.address().hex() });
  } else {
    sendResponse({ error: 'No accounts signed in' });
  }
}

async function connect(account, sendResponse) {
  await Permissions.addDomain(await getCurrentDomain());
  getAccountAddress(account, sendResponse);
}

async function disconnect(sendResponse) {
  await Permissions.removeDomain(await getCurrentDomain());
  sendResponse({});
}

async function isConnected(sendResponse) {
  const status = await Permissions.isDomainAllowed(await getCurrentDomain());
  sendResponse(status);
}

async function signAndSubmitTransaction(client, account, transaction, sendResponse) {
  if (!checkConnected(sendResponse)) {
    return;
  }

  try {
    const signedTransaction = signTransaction(client, account, transaction);
    const response = await client.submitTransaction(account, signedTransaction);
    sendResponse(response);
  } catch (error) {
    sendResponse({ error });
  }
}

async function signTransactionAndSendResponse(client, account, transaction, sendResponse) {
  if (!checkConnected(sendResponse)) {
    return;
  }

  try {
    const signedTransaction = signTransaction(client, account, transaction);
    sendResponse({ signedTransaction });
  } catch (error) {
    sendResponse({ error });
  }
}

async function handleDappRequest(request, sendResponse) {
  const account = await getBackgroundAptosAccountState();
  if (account === undefined) {
    sendResponse({ error: 'No Accounts' });
    return;
  }

  const client = new AptosClient(DEVNET_NODE_URL);
  switch (request.method) {
    case MessageMethod.CONNECT:
      connect(account, sendResponse);
      break;
    case MessageMethod.DISCONNECT:
      disconnect(sendResponse);
      break;
    case MessageMethod.IS_CONNECTED:
      isConnected(sendResponse);
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
    default:
      throw new Error(`${request.method} method is not supported`);
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
