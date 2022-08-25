/* eslint-disable no-console */
import './App.css';
import React, { useEffect, useState } from 'react';

function App() {
  const [address, setAddress] = useState<string | undefined>(undefined);
  const [isConnected, setIsConnected] = useState<boolean | undefined>(undefined);
  const [network, setNetwork] = useState<string | undefined>(undefined);
  const [isSubmittingTransaction, setIsSubmittingTransaction] = useState<boolean>(false);
  const [isSigningTransaction, setIsSigningTransaction] = useState<boolean>(false);
  const [isSigningMessage, setIsSigningMessage] = useState<boolean>(false);

  const transaction = {
    arguments: [address, '717'],
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
        setNetwork(activeNetworkName);
      } else {
        setAddress(undefined);
        setNetwork(undefined);
      }
    }

    window.aptos.onAccountChange(async (account: any) => {
      if (account.address) {
        setIsConnected(true);
        setAddress(account.address);
        setNetwork(await window.aptos.network());
      } else {
        setIsConnected(false);
        setAddress(undefined);
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
      setNetwork(undefined);
    } else {
      const activeAccount = await window.aptos.connect();
      const activeNetworkName = await window.aptos.network();
      setIsConnected(true);
      setAddress(activeAccount.address);
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
        const response = await window.aptos.signMessage('Hello');
        console.log(response);
      } catch (error) {
        console.error(error);
      }
      setIsSigningMessage(false);
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
      </header>
    </div>
  );
}

export default App;
