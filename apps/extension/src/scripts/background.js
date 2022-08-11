// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, BCS, HexString } from 'aptos';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import { sign } from 'tweetnacl';
import { MessageMethod, PermissionType } from '../core/types/dappTypes';
import {
  getBackgroundAptosAccountState,
  getBackgroundNetwork,
  getBackgroundCurrentPublicAccount,
} from '../core/utils/account';
import Permissions from '../core/utils/permissions';
import { DappErrorType } from '../core/types/errors';

// Utils

async function checkAccount(sendResponse) {
  const account = await getBackgroundAptosAccountState();
  if (account === undefined) {
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
  }
  return account;
}

async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const url = new URL(tabs[0].url);
  return url.hostname;
}

async function signTransaction(account, signingMessage) {
  return account.signHexString(signingMessage).hex();
}

async function checkConnected(address, sendResponse) {
  if (Permissions.isDomainAllowed(await getCurrentDomain(), address)) {
    return true;
  }
  sendResponse({ error: DappErrorType.UNAUTHORIZED });
  return false;
}

function rejectRequest(sendResponse) {
  sendResponse({ error: DappErrorType.USER_REJECTION });
}

// Aptos dApp methods

function getAccountAddress(publicAccount, sendResponse) {
  if (!checkConnected(publicAccount.address, sendResponse)) {
    return;
  }
  sendResponse(publicAccount);
}

async function getChainId(client, sendResponse) {
  try {
    const chainId = await client.getChainId();
    sendResponse({ chainId });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function getNetwork(sendResponse) {
  try {
    const network = await getBackgroundNetwork();
    sendResponse(network.name);
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function getSequenceNumber(client, publicAccount, sendResponse) {
  if (!checkConnected(publicAccount.address, sendResponse)) {
    return;
  }

  try {
    const { sequence_number: sequenceNumber } = await client.getAccount(publicAccount.address);
    sendResponse({ sequenceNumber });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function connect(publicAccount, sendResponse) {
  if (await Permissions.requestPermissions(
    PermissionType.CONNECT,
    await getCurrentDomain(),
    publicAccount.address,
  )) {
    getAccountAddress(publicAccount, sendResponse);
  } else {
    rejectRequest(sendResponse);
  }
}

async function disconnect(address, sendResponse) {
  await Permissions.removeDomain(await getCurrentDomain(), address);
  sendResponse({});
}

async function isConnected(address, sendResponse) {
  const status = await Permissions.isDomainAllowed(
    await getCurrentDomain(),
    address,
  );
  sendResponse(status);
}

async function submitTransaction(client, publicAccount, signedTransaction, sendResponse) {
  const account = await checkAccount(sendResponse);
  if (!checkConnected(publicAccount.address, sendResponse) || account === undefined) {
    return;
  }

  try {
    const response = await client.submitSignedBCSTransaction(
      new HexString(signedTransaction).toUint8Array(),
    );
    sendResponse(response);
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function signTransactionAndSendResponse(publicAccount, signingMessage, sendResponse) {
  const account = await checkAccount(sendResponse);
  if (!checkConnected(publicAccount.address, sendResponse) || account === undefined) {
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
    const signature = await signTransaction(account, signingMessage);
    sendResponse({ signature });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function signMessage(publicAccount, message, sendResponse) {
  const account = await checkAccount(sendResponse);
  if (!checkConnected(publicAccount.address, sendResponse) || account === undefined) {
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
  const publicAccount = await getBackgroundCurrentPublicAccount();
  const network = await getBackgroundNetwork();
  console.log('handleDappRequest', publicAccount, network);
  if (publicAccount === undefined) {
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
    return;
  }

  // The fetch adapter is neccessary to use axios from a service worker
  const client = new AptosClient(network.nodeUrl, { adapter: fetchAdapter });
  switch (request.method) {
    case MessageMethod.CONNECT:
      connect(publicAccount, sendResponse);
      break;
    case MessageMethod.DISCONNECT:
      disconnect(publicAccount.address, sendResponse);
      break;
    case MessageMethod.IS_CONNECTED:
      isConnected(publicAccount.address, sendResponse);
      break;
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      getAccountAddress(publicAccount, sendResponse);
      break;
    case MessageMethod.GET_CHAIN_ID:
      getChainId(client, sendResponse);
      break;
    case MessageMethod.GET_NETWORK:
      getNetwork(sendResponse);
      break;
    case MessageMethod.GET_SEQUENCE_NUMBER:
      getSequenceNumber(client, publicAccount, sendResponse);
      break;
    case MessageMethod.SUBMIT_TRANSACTION:
      submitTransaction(client, publicAccount, request.args.signedTransaction, sendResponse);
      break;
    case MessageMethod.SIGN_TRANSACTION:
      signTransactionAndSendResponse(publicAccount, request.args.signingMessage, sendResponse);
      break;
    case MessageMethod.SIGN_MESSAGE:
      signMessage(publicAccount, request.args.message, sendResponse);
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
