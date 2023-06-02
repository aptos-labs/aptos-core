import React from "react";

import { PetraWallet } from "petra-plugin-wallet-adapter";

import { AptosWalletAdapterProvider } from "@aptos-labs/wallet-adapter-react";
import Home from "./Home";

function App() {
  return (
    <AptosWalletAdapterProvider
      plugins={[new PetraWallet()]}
      autoConnect={true}
    >
      <Home />
    </AptosWalletAdapterProvider>
  );
}

export default App;
