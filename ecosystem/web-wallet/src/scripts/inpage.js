// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MessageMethod } from '../core/types'

class Web3 {
  requestId

  constructor () {
    this.requestId = 0
  }

  connect () {
    return this._message(MessageMethod.CONNECT, {})
  }

  disconnect () {
    return this._message(MessageMethod.DISCONNECT, {})
  }

  isConnected () {
    return this._message(MessageMethod.IS_CONNECTED, {})
  }

  account () {
    return this._message(MessageMethod.GET_ACCOUNT_ADDRESS, {})
  }

  signAndSubmitTransaction (transaction) {
    return this._message(MessageMethod.SIGN_AND_SUBMIT_TRANSACTION, { transaction })
  }

  signTransaction (transaction) {
    return this._message(MessageMethod.SIGN_TRANSACTION, { transaction })
  }

  _message(method, args) {
    const id = this.requestId++
    return new Promise(function (resolve, reject) {
      window.postMessage({ method, args, id })
      window.addEventListener('message', function handler (event) {
        if (event.data.responseMethod === method &&
            event.data.id === id) {
          const response = event.data.response
          this.removeEventListener('message', handler)
          if (response.error) {
            reject(response.error ?? 'Error')
          } else {
            resolve(response)
          }
        }
      })
    })
  }
}

window.aptos = new Web3()
