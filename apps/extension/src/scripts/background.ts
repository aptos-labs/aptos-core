// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount,
  AptosClient,
  BCS,
} from 'aptos';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import axios from 'axios';
import { sign } from 'tweetnacl';
import { MessageMethod, Permission, warningPrompt } from 'core/types/dappTypes';
import PromptPresenter from 'core/utils/promptPresenter';
import {
  getBackgroundAptosAccountState,
  getBackgroundNetwork,
  getBackgroundCurrentPublicAccount,
} from 'core/utils/account';
import Permissions from 'core/utils/permissions';
import { DappErrorType, TransactionError } from 'core/types/errors';
import { PublicAccount } from 'core/types/stateTypes';
import { TransactionPayload } from 'aptos/dist/generated';

// The fetch adapter is necessary to use axios from a service worker
axios.defaults.adapter = fetchAdapter;

type ResponseCallback = (response?: any) => void;

// Utils

async function checkAccount(sendResponse: ResponseCallback) {
  const account = await getBackgroundAptosAccountState();
  if (account === undefined) {
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
  }
  return account;
}

async function getCurrentDomain() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tabs[0].url === undefined) {
    throw new Error("Couldn't retrieve tab URL");
  }
  const url = new URL(tabs[0].url);
  return url.hostname;
}

async function signTransaction(
  client: AptosClient,
  account: AptosAccount,
  transaction: TransactionPayload,
) {
  const address = account.address();
  const txn = await client.generateTransaction(address, transaction);
  return client.signTransaction(account, txn);
}

async function checkConnected(address: string, sendResponse: ResponseCallback) {
  if (await Permissions.isDomainAllowed(await getCurrentDomain(), address)) {
    return true;
  }
  sendResponse({ error: DappErrorType.UNAUTHORIZED });
  return false;
}

function rejectRequest(sendResponse: ResponseCallback) {
  sendResponse({ error: DappErrorType.USER_REJECTION });
}

// Aptos dApp methods

async function getAccountAddress(publicAccount: PublicAccount, sendResponse: ResponseCallback) {
  const connected = await checkConnected(publicAccount.address, sendResponse);
  if (!connected) {
    return;
  }
  sendResponse(publicAccount);
}

async function getNetwork(sendResponse: ResponseCallback) {
  try {
    const network = await getBackgroundNetwork();
    sendResponse(network.name);
  } catch (error) {
    sendResponse({ data: error, error: DappErrorType.INTERNAL_ERROR });
  }
}

async function connect(publicAccount: PublicAccount, sendResponse: ResponseCallback) {
  if (await Permissions.requestPermissions(
    Permission.CONNECT,
    await getCurrentDomain(),
    publicAccount.address,
  )) {
    await getAccountAddress(publicAccount, sendResponse);
  } else {
    rejectRequest(sendResponse);
  }
}

async function disconnect(address: string, sendResponse: ResponseCallback) {
  await Permissions.removeDomain(await getCurrentDomain(), address);
  sendResponse({});
}

async function isConnected(address: string, sendResponse: ResponseCallback) {
  const status = await Permissions.isDomainAllowed(
    await getCurrentDomain(),
    address,
  );
  sendResponse(status);
}

async function signAndSubmitTransaction(
  client: AptosClient,
  publicAccount: PublicAccount,
  transaction: TransactionPayload,
  sendResponse: ResponseCallback,
) {
  const connected = await checkConnected(publicAccount.address, sendResponse);
  if (!connected) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    Permission.SIGN_AND_SUBMIT_TRANSACTION,
    await getCurrentDomain(),
    publicAccount.address,
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }

  const account = await checkAccount(sendResponse);
  if (!account) {
    return;
  }

  try {
    const signedTransaction = await signTransaction(client, account, transaction);
    const response = await client.submitTransaction(signedTransaction);
    sendResponse(response);
  } catch (error: any) {
    sendResponse(TransactionError(error));
  }
}

async function signTransactionAndSendResponse(
  client: AptosClient,
  publicAccount: PublicAccount,
  transaction: TransactionPayload,
  sendResponse: ResponseCallback,
) {
  const connected = await checkConnected(publicAccount.address, sendResponse);
  if (!connected) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    Permission.SIGN_TRANSACTION,
    await getCurrentDomain(),
    publicAccount.address,
  );
  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }

  const account = await checkAccount(sendResponse);
  if (!account) {
    return;
  }

  try {
    const signedTransaction = await signTransaction(client, account, transaction);
    sendResponse({ signedTransaction });
  } catch (error: any) {
    sendResponse(TransactionError(error));
  }
}

async function signMessage(
  publicAccount: PublicAccount,
  message: string,
  sendResponse: ResponseCallback,
) {
  const connected = await checkConnected(publicAccount.address, sendResponse);
  if (!connected) {
    return;
  }

  const permission = await Permissions.requestPermissions(
    Permission.SIGN_MESSAGE,
    await getCurrentDomain(),
    publicAccount.address,
  );

  if (!permission) {
    rejectRequest(sendResponse);
    return;
  }

  const account = await checkAccount(sendResponse);
  if (!account) {
    return;
  }

  try {
    const serializer = new BCS.Serializer();
    serializer.serializeStr(message);
    const signature = sign(serializer.getBytes(), account.signingKey.secretKey);
    const signedMessage = Buffer.from(signature).toString('hex');
    sendResponse({ signedMessage });
  } catch (error) {
    sendResponse({ error });
  }
}

function shouldShowNoAccountsPrompt(method: MessageMethod) {
  switch (method) {
    case MessageMethod.CONNECT:
      return true;
    default:
      return false;
  }
}

async function handleDappRequest(request: any, sendResponse: ResponseCallback) {
  const publicAccount = await getBackgroundCurrentPublicAccount();
  const network = await getBackgroundNetwork();
  if (!publicAccount) {
    if (shouldShowNoAccountsPrompt(request.method)) {
      await PromptPresenter.promptUser(warningPrompt());
    }
    sendResponse({ error: DappErrorType.NO_ACCOUNTS });
    return;
  }

  const client = new AptosClient(network.nodeUrl);
  switch (request.method) {
    case MessageMethod.CONNECT:
      await connect(publicAccount, sendResponse);
      break;
    case MessageMethod.DISCONNECT:
      await disconnect(publicAccount.address, sendResponse);
      break;
    case MessageMethod.IS_CONNECTED:
      await isConnected(publicAccount.address, sendResponse);
      break;
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      await getAccountAddress(publicAccount, sendResponse);
      break;
    case MessageMethod.GET_NETWORK:
      await getNetwork(sendResponse);
      break;
    case MessageMethod.SIGN_AND_SUBMIT_TRANSACTION:
      await signAndSubmitTransaction(
        client,
        publicAccount,
        request.args.transaction,
        sendResponse,
      );
      break;
    case MessageMethod.SIGN_TRANSACTION:
      await signTransactionAndSendResponse(
        client,
        publicAccount,
        request.args.transaction,
        sendResponse,
      );
      break;
    case MessageMethod.SIGN_MESSAGE:
      await signMessage(publicAccount, request.args.message, sendResponse);
      break;
    default:
      // method not supported
      break;
  }
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  try {
    handleDappRequest(request, sendResponse).then();
  } catch (error) {
    sendResponse({ error });
  }
  return true;
});
