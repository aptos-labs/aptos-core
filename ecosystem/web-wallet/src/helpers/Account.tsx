import React from 'react'
import { AptosAccount } from 'aptos'
import { Buffer } from 'buffer'
import { keyLength } from '../constants'
import { Result, err, ok } from '../types'
import { GlobalState } from '../GlobalState'

export function loginAccount (key: string, dispatch: React.Dispatch<GlobalState>): Result<AptosAccount, Error> {
  if (key.length === keyLength) {
    try {
      const encodedKey = Uint8Array.from(Buffer.from(key, 'hex'))
      const account = new AptosAccount(encodedKey, undefined)
      dispatch({ account })
      return ok(account)
    } catch (e) {
      return err(e as Error)
    }
  } else {
    return err(new Error('Key not the correct the length'))
  }
}
