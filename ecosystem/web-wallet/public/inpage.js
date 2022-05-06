// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

class Web3 {
  requestId

  constructor () {
    this.requestId = 0
  }

  account () {
    const id = this.requestId++
    return new Promise(function (resolve, reject) {
      const method = 'getAccountAddress'
      window.postMessage({ method, id })
      window.addEventListener('message', function handler (event) {
        if (event.data.responseMethod === method &&
            event.data.id === id) {
          const response = event.data.response
          this.removeEventListener('message', handler)
          if (response.address) {
            resolve(response.address)
          } else {
            reject(response.error ?? 'Error')
          }
        }
      })
    })
  }

  signAndSubmitTransaction (transaction) {
    const id = this.requestId++
    return new Promise(function (resolve, reject) {
      const method = 'signTransaction'
      window.postMessage({ method, transaction, id })
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
