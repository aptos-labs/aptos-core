// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosClient, Types } from 'aptos'
import { DEVNET_NODE_URL } from './constants'
import { MessageMethod } from './types'
import { getAptosAccountState } from './utils/account'

chrome.runtime.onMessage.addListener(function (request, sender, sendResponse) {
  const account = getAptosAccountState()
  if (account === undefined) {
    sendResponse({ error: 'No Accounts' })
    return
  }
  switch (request.method) {
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      getAccountAddress(account, sendResponse)
      break
    case MessageMethod.SIGN_TRANSACTION:
      signTransaction(account, request.transaction, sendResponse)
      break
    default:
      // method not handled
      break
  }
  return true
})

function getAccountAddress (account: AptosAccount, sendResponse: (response?: any) => void) {
  if (account.address()) {
    sendResponse({ address: account.address().hex() })
  } else {
    sendResponse({ error: 'No accounts signed in' })
  }
}

async function signTransaction (account: AptosAccount, transaction: Types.UserTransactionRequest, sendResponse: (response?: any) => void) {
  const client = new AptosClient(DEVNET_NODE_URL)
  const message = await client.createSigningMessage(transaction)
  const signatureHex = account.signHexString(message.substring(2))
  const transactionSignature = {
    type: 'ed25519_signature',
    public_key: account.pubKey().hex(),
    signature: signatureHex.hex()
  }
  sendResponse({ transaction: { signature: transactionSignature, ...transaction } })
}
