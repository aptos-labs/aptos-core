import { AptosAccount } from 'aptos'
import { Buffer } from 'buffer'
import { keyLength } from '../constants'
import { Result, err, ok } from '../types'

export function loginAccount (key: string): Result<AptosAccount, Error> {
  if (key.length === keyLength) {
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
