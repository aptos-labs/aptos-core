import React, { useState } from 'react'
import { Buffer } from 'buffer'
import { KEY_LENGTH } from '../constants'
import { useNavigate } from 'react-router-dom'
import { useGlobalState } from '../GlobalState'
import { createNewAccount, loginAccount } from '../utils/account'

import './App.css'

export default function Login () {
  const [key, setKey] = useState('')
  const [error, setError] = useState('')
  const [, dispatch] = useGlobalState()
  const navigate = useNavigate()

  function handleSubmit (event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const result = loginAccount(key)
    if (result.isOk()) {
      const account = result.value
      dispatch({ account })
      navigate('/wallet')
    } else {
      setError(result.error.message)
    }
  }

  function onChange (event: React.ChangeEvent<HTMLInputElement>) {
    setKey(event.target.value)
    setError('')
  }

  function onGenerateClick (event: React.MouseEvent<HTMLButtonElement>) {
    const result = createNewAccount()
    if (result.isOk()) {
      const account = result.value
      const accountKey = Buffer.from(account.signingKey.secretKey.buffer).toString('hex').slice(0, KEY_LENGTH)
      setKey(accountKey)
    } else {
      setError(result.error.message)
    }
  }

  return (
    <div className="App-header">
      <h2>Aptos Wallet</h2>
      <form onSubmit={handleSubmit}>
        <input onChange={onChange} value={key}/>
      </form>
      <small className="Error-message">{error}</small>
      <button onClick={onGenerateClick}>
          Generate Account
      </button>
    </div>
  )
}
