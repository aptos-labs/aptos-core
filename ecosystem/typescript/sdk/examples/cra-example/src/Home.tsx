import React, { useEffect } from "react";
import { Network, Provider } from "aptos";
import { WalletSelector } from "@aptos-labs/wallet-adapter-ant-design";
import axios from "axios";

const DYNAMIC_URL = "https://api.restful-api.dev/objects";

function Home() {
  useEffect(() => {
    const getResources = async () => {
      const provider = new Provider(Network.TESTNET);
      return await provider.getAccountResources("0x1");
    };
    getResources().then((data) => console.log(data));

    axios
      .post(DYNAMIC_URL, {
        name: "Apple MacBook Pro 16",
        data: {
          year: 2019,
          price: 1849.99,
          "CPU model": "Intel Core i9",
          "Hard disk size": "1 TB",
        },
      })
      .then(function (response) {
        console.log(response);
      })
      .catch(function (error) {
        console.log(error);
      });
  }, []);

  return (
    <div>
      <WalletSelector />
    </div>
  );
}

export default Home;
