import * as React from "react";
import { DappSite } from "./DappSite";

function App() {
  const aptos = (window as any).aptos;
  const location = window.location;

  const [userAddress, setUserAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (!aptos) return;
    aptos.account().then((account: string) => {
      setUserAddress(account);
      if (!address) setAddress(account);
    });
  }, [aptos]);

  const [address, setAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    setAddress(location.pathname.slice(1).toLowerCase());
  }, [location.pathname]);

  return (
    <div className="App">
      {address != null ? (
        <DappSite userAddress={userAddress} address={address} />
      ) : (
        <p>Loading...</p>
      )}
    </div>
  );
}

export default App;
