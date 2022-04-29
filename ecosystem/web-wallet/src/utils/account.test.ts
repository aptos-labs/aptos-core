// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { loginAccount } from '../utils/account'
import { KEY_LENGTH } from '../constants'

test('test login fail with empty key', () => {
  const response = loginAccount('')
  expect(response.isErr()).toBe(true)
})

test('test login fail with long key', () => {
  let key = 'A_really_long_key'
  while (key.length <= KEY_LENGTH) {
    key += key
  }
  const response = loginAccount(key)
  expect(response.isErr()).toBe(true)
})
