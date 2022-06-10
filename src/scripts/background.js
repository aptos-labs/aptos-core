// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from 'aptos'
import { DEVNET_NODE_URL } from '../core/constants'
import { MessageMethod } from '../core/types'
import { getBackgroundAptosAccountState } from '../core/utils/account'

chrome.runtime.onMessage.addListener(function (request, sender, sendResponse) {
  try {
    handleDappRequest(request, sendResponse)
  } catch (error) {
    sendResponse({ error })
  }
  return true
})

async function handleDappRequest (request, sendResponse) {
  const account = await getBackgroundAptosAccountState()
  if (account === undefined) {
    sendResponse({ error: 'No Accounts' })
    return
  }

  const client = new AptosClient(DEVNET_NODE_URL)
  switch (request.method) {
    case MessageMethod.CONNECT:
      connect(account, sendResponse)
      break
    case MessageMethod.DISCONNECT:
      disconnect()
      break
    case MessageMethod.IS_CONNECTED:
      isConnected(sendResponse)
      break
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      getAccountAddress(account, sendResponse)
      break
    case MessageMethod.SIGN_AND_SUBMIT_TRANSACTION:
      signAndSubmitTransaction(client, account, request.args.transaction, sendResponse)
      break
    case MessageMethod.SIGN_TRANSACTION:
      signTransactionAndSendResponse(client, account, request.args.transaction, sendResponse)
      break
    default:
      throw new Error(request.method + ' method is not supported')
  }
}

function connect (account, sendResponse) {
  // todo: register caller for permission checking purposes
  getAccountAddress(account, sendResponse)
}

function disconnect () {
  // todo: unregister caller
}

function isConnected (sendResponse) {
  // todo: send boolean response based on registered caller
  sendResponse(true)
}

function getAccountAddress (account, sendResponse) {
  if (account.address()) {
    sendResponse({ address: account.address().hex() })
  } else {
    sendResponse({ error: 'No accounts signed in' })
  }
}

async function signAndSubmitTransaction (client, account, transaction, sendResponse) {
  try {
    const signedTransaction = signTransaction(client, account, transaction)
    const response = await client.submitTransaction(account, signedTransaction)
    sendResponse(response)
  } catch (error) {
    sendResponse({ error })
  }
}

async function signTransactionAndSendResponse (client, account, transaction, sendResponse) {
  try {
    const signedTransaction = signTransaction(client, account, transaction)
    sendResponse({ signedTransaction })
  } catch (error) {
    sendResponse({ error })
  }
}

async function signTransaction (client, account, transaction) {
  const address = account.address()
  const txn = await client.generateTransaction(address, transaction)
  return await client.signTransaction(account, txn)
}
