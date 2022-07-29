import './App.css';
import React, { useEffect, useState } from 'react';

function App() {
  const [address, setAddress] = useState<string | undefined>(undefined);
  const [isConnected, setIsConnected] = useState<boolean | undefined>(undefined);

  useEffect(() => {
    window.aptos.on('accountChanged', (account: any) => {
      if (account.address) {
        setIsConnected(true);
        setAddress(account.address);
      } else {
        setAddress(undefined);
        setIsConnected(true);
      }
    });

    const fetchStatus = async () => {
      const flag = await window.aptos.isConnected();
      if (flag) {
        const account = await window.aptos.account();
        setAddress(account.address);
      }
      setIsConnected(flag);
    };
    fetchStatus();
  });

  const onConnectClick = async () => {
    if (isConnected) {
      await window.aptos.disconnect();
      setIsConnected(false);
      setAddress(undefined);
    } else {
      const result = await window.aptos.connect();
      setAddress(result.address);
      setIsConnected(true);
    }
  };

  return (
    <div className="App">
      <header className="App-header">
        <p>
          {isConnected ? `Address: ${address}` : 'Not Connected'}
        </p>
        <button className="Button" type="button" onClick={onConnectClick}>{isConnected ? 'Disconnect' : 'Connect'}</button>
      </header>
    </div>
  );
}

export default App;
