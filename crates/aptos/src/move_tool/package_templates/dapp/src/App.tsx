import React from 'react';
import {AptosClient} from "aptos";

const aptosCoinStore = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

// Create an AptosClient to interact with mainnet.
const client = new AptosClient('https://fullnode.mainnet.aptoslabs.com/v1');

function App() {
  const [address, setAddress] = React.useState<string | undefined>(undefined);
  const [balance, setBalance] = React.useState<number | undefined>(undefined);

  const handleClick = async () => {
    if (address) {
      let resources = await client.getAccountResources(address);
      let accountResource = resources.find((r) => r.type === aptosCoinStore);
      setBalance(parseInt((accountResource?.data as any).coin.value));
    } else {
      setBalance(0);
    }
  }

  return (
    <div className="app">
      <div className="app-label">Balance of APT coin</div>
      <table className="app-table">
        <tr>
          <td width="20%" className="app-name">Address:</td>
          <td width="80%"><input className="app-input" id="usdt" value={address}
                                 onChange={e => setAddress(e.target.value)}/></td>
        </tr>
        <tr>
          <td colSpan={2} className="app-button">
            <input type="button" className="button" width="200" value="Check Balance" onClick={handleClick}/>
          </td>
        </tr>
        <tr>
          <td colSpan={2} className="app-result">
            <div className="app-text">{balance}</div>
          </td>
        </tr>
      </table>
    </div>
  );
}

export default App;
