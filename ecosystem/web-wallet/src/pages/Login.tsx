import React, { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useGlobalState } from '../GlobalState'
import { loginAccount } from '../utils/account'

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

  return (
    <div className="App-header">
      <h2>Aptos Wallet</h2>
      <form onSubmit={handleSubmit}>
        <input onChange={onChange}/>
      </form>
      <text className="Error-message">{error}</text>
    </div>
  )
}
