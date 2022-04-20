import React, {useState} from "react";
import { AptosAccount } from "aptos";
import { Buffer } from 'buffer'
import { useNavigate } from "react-router-dom";
import { useGlobalState } from "../GlobalState";

import './App.css';

export default function Login() {
  const [key, setKey] = useState('');
  const [, dispatch] = useGlobalState();
  const navigate = useNavigate();

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    let encodedKey = Uint8Array.from(Buffer.from(key, 'hex'))
    const account = new AptosAccount(encodedKey, undefined);
    dispatch({account: account});
    navigate('/wallet');
  }

  return (
    <div className="App-header">
      <h2>Aptos Wallet</h2>
      <form onSubmit={handleSubmit}>
        <input onChange={(e) => setKey(e.target.value)}/>
      </form>
    </div>
  );
}
