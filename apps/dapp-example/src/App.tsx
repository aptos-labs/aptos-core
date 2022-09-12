/* eslint-disable no-console */
import './App.css';
import React, { useEffect, useState } from 'react';
import nacl from 'tweetnacl';
import { Buffer } from 'buffer';

function App() {
  const [address, setAddress] = useState<string | undefined>(undefined);
  const [publicKey, setPublicKey] = useState<string | undefined>(undefined);
  const [isConnected, setIsConnected] = useState<boolean | undefined>(undefined);
  const [network, setNetwork] = useState<string | undefined>(undefined);
  const [isSubmittingTransaction, setIsSubmittingTransaction] = useState<boolean>(false);
  const [isSigningTransaction, setIsSigningTransaction] = useState<boolean>(false);
  const [isSigningMessage, setIsSigningMessage] = useState<boolean>(false);
  const [isVerifying, setIsVerifying] = useState<boolean>(false);

  const transaction = {
    arguments: [address, 717],
    function: '0x1::coin::transfer',
    type: 'entry_function_payload',
    type_arguments: ['0x1::aptos_coin::AptosCoin'],
  };

  useEffect(() => {
    async function fetchStatus() {
      const isAlreadyConnected = await window.aptos.isConnected();
      setIsConnected(isAlreadyConnected);
      if (isAlreadyConnected) {
        const [activeAccount, activeNetworkName] = await Promise.all([
          window.aptos.account(),
          window.aptos.network(),
        ]);
        setAddress(activeAccount.address);
        setPublicKey(activeAccount.publicKey);
        setNetwork(activeNetworkName);
      } else {
        setAddress(undefined);
        setPublicKey(undefined);
        setNetwork(undefined);
      }
    }

    window.aptos.onAccountChange(async (account: any) => {
      if (account.address) {
        setIsConnected(true);
        setAddress(account.address);
        setPublicKey(account.publicKey);
        setNetwork(await window.aptos.network());
      } else {
        setIsConnected(false);
        setAddress(undefined);
        setPublicKey(undefined);
        setNetwork(undefined);
      }
    });

    window.aptos.onNetworkChange((params: any) => {
      setNetwork(params.networkName);
    });

    fetchStatus();
  }, []);

  const onConnectClick = async () => {
    if (isConnected) {
      await window.aptos.disconnect();
      setIsConnected(false);
      setAddress(undefined);
      setPublicKey(undefined);
      setNetwork(undefined);
    } else {
      const activeAccount = await window.aptos.connect();
      const activeNetworkName = await window.aptos.network();
      setIsConnected(true);
      setAddress(activeAccount.address);
      setPublicKey(activeAccount.publicKey);
      setNetwork(activeNetworkName);
    }
  };

  const onSubmitTransactionClick = async () => {
    if (!isSubmittingTransaction) {
      setIsSubmittingTransaction(true);
      try {
        const pendingTransaction = await window.aptos.signAndSubmitTransaction(transaction);
        console.log(pendingTransaction);
      } catch (error) {
        console.error(error);
      }
      setIsSubmittingTransaction(false);
    }
  };

  const onSignTransactionClick = async () => {
    if (!isSubmittingTransaction) {
      setIsSigningTransaction(true);
      try {
        const signedTransaction = await window.aptos.signTransaction(transaction);
        console.log(signedTransaction);
      } catch (error) {
        console.error(error);
      }
      setIsSigningTransaction(false);
    }
  };

  const onSignMessageClick = async () => {
    if (!isSigningMessage && address) {
      setIsSigningMessage(true);
      try {
        const response = await window.aptos.signMessage({
          address: true,
          application: true,
          chainId: true,
          message: 'Hello',
          nonce: Date.now().toString(),
        });
        console.log(response);
      } catch (error) {
        console.error(error);
      }
      setIsSigningMessage(false);
    }
  };

  const onVerifyClick = async () => {
    if (!isVerifying && address) {
      setIsVerifying(true);
      try {
        const nonce = Date.now().toString();
        const response = await window.aptos.signMessage({
          message: 'Hello',
          nonce,
        });
        // Remove the 0x prefix
        const key = publicKey!.slice(2, 66);
        const verified = nacl.sign.detached.verify(Buffer.from(response.fullMessage), Buffer.from(response.signature, 'hex'), Buffer.from(key, 'hex'));
        console.log(verified);
      } catch (error) {
        console.error(error);
      }
      setIsVerifying(false);
    }
  };

  return (
    <div className="App">
      <header className="App-header">
        <p>
          {isConnected ? `Address: ${address}` : 'Not Connected'}
        </p>
        <p>
          {`Network: ${network}`}
        </p>
        <button className="Button" type="button" onClick={onConnectClick}>{isConnected ? 'Disconnect' : 'Connect'}</button>
        <button className="Button" type="button" onClick={onSubmitTransactionClick}>{isSubmittingTransaction ? 'Submitting...' : 'Submit Transaction'}</button>
        <button className="Button" type="button" onClick={onSignTransactionClick}>{isSigningTransaction ? 'Sigining...' : 'Sign Transaction'}</button>
        <button className="Button" type="button" onClick={onSignMessageClick}>{isSigningMessage ? 'Signing...' : 'Sign Message'}</button>
        <button className="Button" type="button" onClick={onVerifyClick}>{isVerifying ? 'Verifying...' : 'Verify Message'}</button>
      </header>
    </div>
  );
}

export default App;
