// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosClient, Types } from 'aptos'
import { devnetNodeUrl } from './constants'
import { MessageMethod } from './types'
import { getAptosAccountState } from './utils/account'

chrome.runtime.onMessageExternal.addListener(async function (request, _sender, sendResponse) {
  const account = getAptosAccountState()
  if (account === undefined) {
    sendResponse({ error: 'No Accounts' })
    return
  }
  switch (request.method) {
    case MessageMethod.GET_ACCOUNT_ADDRESS:
      getAccountAddress(account, sendResponse)
      break;
    case MessageMethod.SIGN_TRANSACTION:
      signTransaction(account, request.transaction, sendResponse)
      break;
    default:
      throw(request.method + ' method is not supported')
  }
})

function getAccountAddress( account: AptosAccount, sendResponse: (response?: any) => void) {
  if (account.address()) {
    sendResponse({ address: account.address().hex() })
  } else {
    sendResponse({ error: 'No accounts signed in' })
  }
}

async function signTransaction (account: AptosAccount, transaction: Types.UserTransactionRequest, sendResponse: (response?: any) => void) {
  const client = new AptosClient(devnetNodeUrl)
  const message = await client.createSigningMessage(transaction)
  const signatureHex = account.signHexString(message.substring(2))
  const transactionSignature = {
    type: 'ed25519_signature',
    public_key: account.pubKey().hex(),
    signature: signatureHex.hex()
  }
  sendResponse({ transaction: { signature: transactionSignature, ...transaction } })
}
