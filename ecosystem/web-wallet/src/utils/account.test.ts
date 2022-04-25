import { loginAccount } from '../utils/account'
import { keyLength } from '../constants'

test("test login fail with empty key", () => {
  let response = loginAccount('')
  expect(response.isErr()).toBe(true)
});

test("test login fail with long key", () => {
  let key = 'A_really_long_key'
  while (key.length <= keyLength) {
    key += key  
  }
  let response = loginAccount(key)
  expect(response.isErr()).toBe(true)
});