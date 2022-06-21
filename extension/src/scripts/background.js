// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from 'aptos';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import { DEVNET_NODE_URL } from '../core/constants';
import { MessageMethod, PermissionType } from '../core/types';
import { getBackgroundAptosAccountState } from '../core/utils/account';
import Permissions from '../core/utils/permissions';

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

async function checkConnected(sendResponse) {
  if (Permissions.isDomainAllowed(await getCurrentDomain())) {
    return true;
  }
  sendResponse({ error: 'App not connected - call aptos.connect()' });
  return false;
}

function rejectRequest(sendResponse) {
  sendResponse({ error: 'User rejected request' });
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
  if (await Permissions.requestPermissions(PermissionType.CONNECT, await getCurrentDomain())) {
    getAccountAddress(account, sendResponse);
  } else {
    rejectRequest(sendResponse);
  }
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

  const permission = await Permissions.requestPermissions(
    PermissionType.SIGN_AND_SUBMIT_TRANSACTION,
    await getCurrentDomain(),
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
    sendResponse({ error });
  }
}

async function signTransactionAndSendResponse(client, account, transaction, sendResponse) {
  if (!checkConnected(sendResponse)) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    PermissionType.SIGN_TRANSACTION,
    await getCurrentDomain(),
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }
  try {
    const signedTransaction = await signTransaction(client, account, transaction);
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

  // The fetch adapter is neccessary to use axios from a service worker
  const client = new AptosClient(DEVNET_NODE_URL, { adapter: fetchAdapter });
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
