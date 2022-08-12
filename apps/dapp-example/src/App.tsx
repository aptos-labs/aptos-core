/* eslint-disable no-console */
import './App.css';
import React, { useEffect, useState } from 'react';
import { BCS, TxnBuilderTypes } from 'aptos';

function App() {
  const [address, setAddress] = useState<string | undefined>(undefined);
  const [isConnected, setIsConnected] = useState<boolean | undefined>(undefined);
  const [network, setNetwork] = useState<string | undefined>(undefined);
  const [isSubmittingTransaction, setIsSubmittingTransaction] = useState<boolean>(false);
  const [isSigningMessage, setIsSigningMessage] = useState<boolean>(false);

  useEffect(() => {
    window.aptos.on('accountChanged', (account: any) => {
      if (account.address) {
        setIsConnected(true);
        setAddress(account.address);
      } else {
        setIsConnected(true);
        setAddress(undefined);
      }
    });

    window.aptos.on('networkChanged', (newNetwork: string) => {
      setNetwork(newNetwork);
    });

    const fetchStatus = async () => {
      const flag = await window.aptos.isConnected();
      if (flag) {
        const account = await window.aptos.account();
        setAddress(account.address);
        setNetwork(await window.aptos.network());
      }
      setIsConnected(flag);
    };
    fetchStatus();
  }, []);

  const onConnectClick = async () => {
    if (isConnected) {
      await window.aptos.disconnect();
      setIsConnected(false);
      setAddress(undefined);
      setNetwork(undefined);
    } else {
      const result = await window.aptos.connect();
      setIsConnected(true);
      setAddress(result.address);
      setNetwork(await window.aptos.network());
    }
  };

  const onSubmitTransactionClick = async () => {
    if (!isSubmittingTransaction && address) {
      setIsSubmittingTransaction(true);
      const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString('0x1::aptos_coin::AptosCoin'));
      const transaction = new TxnBuilderTypes.TransactionPayloadScriptFunction(
        TxnBuilderTypes.ScriptFunction.natural(
          '0x1::coin',
          'transfer',
          [token],
          [
            BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(address)),
            BCS.bcsSerializeUint64(717),
          ],
        ),
      );
      try {
        const pendingTransaction = await window.aptos.signAndSubmitTransaction(transaction);
        console.log(pendingTransaction);
      } catch (error) {
        console.error(error);
      }
      setIsSubmittingTransaction(false);
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
        <button className="Button" type="button" onClick={onSignMessageClick}>{isSigningMessage ? 'Signing...' : 'Sign Message'}</button>
      </header>
    </div>
  );
}

export default App;
