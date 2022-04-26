import { AptosAccount } from 'aptos'
import { Buffer } from 'buffer'
import { KEY_LENGTH } from '../constants'
import { Result, err, ok } from '../types'

export function loginAccount (key: string): Result<AptosAccount, Error> {
  if (key.length === KEY_LENGTH) {
    try {
      const encodedKey = Uint8Array.from(Buffer.from(key, 'hex'))
      // todo: Ping API to check if a legit account
      const account = new AptosAccount(encodedKey, undefined)
      return ok(account)
    } catch (e) {
      return err(e as Error)
    }
  } else {
    return err(new Error('Key not the correct the length'))
  }
}

export function createNewAccount (): Result<AptosAccount, Error> {
  const account = new AptosAccount()
  // todo: make request to create account on chain
  return ok(account)
}
