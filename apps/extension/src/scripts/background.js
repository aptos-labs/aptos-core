// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, BCS, HexString } from 'aptos';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import { sign } from 'tweetnacl';
import { MessageMethod, PermissionType } from '../core/types/dappTypes';
import { getBackgroundAptosAccountState, getBackgroundNodeUrl, getBackgroundNetworkName } from '../core/utils/account';
import Permissions from '../core/utils/permissions';
import { DappErrorType } from '../core/types/errors';

// Utils

async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const url = new URL(tabs[0].url);
  return url.hostname;
}

async function signTransaction(account, signingMessage) {
  return account.signHexString(signingMessage).hex();
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
    const network = await getBackgroundNetworkName();
    sendResponse(network);
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
  }
}

async function getSequenceNumber(client, account, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
    return;
  }

  try {
    const address = account.address().hex();
    const { sequence_number: sequenceNumber } = await client.getAccount(address);
    sendResponse({ sequenceNumber });
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.TRANSACTION_FAILURE });
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

async function submitTransaction(client, account, signedTransaction, sendResponse) {
  if (!checkConnected(account, sendResponse)) {
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

async function signTransactionAndSendResponse(account, signingMessage, sendResponse) {
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
    const signature = await signTransaction(account, signingMessage);
    sendResponse({ signature });
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
    case MessageMethod.GET_CHAIN_ID:
      getChainId(client, sendResponse);
      break;
    case MessageMethod.GET_NETWORK:
      getNetwork(sendResponse);
      break;
    case MessageMethod.GET_SEQUENCE_NUMBER:
      getSequenceNumber(client, account, sendResponse);
      break;
    case MessageMethod.SUBMIT_TRANSACTION:
      submitTransaction(client, account, request.args.signedTransaction, sendResponse);
      break;
    case MessageMethod.SIGN_TRANSACTION:
      signTransactionAndSendResponse(account, request.args.signingMessage, sendResponse);
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
