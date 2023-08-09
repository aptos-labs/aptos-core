import React, { useEffect, useState } from 'react';
import './App.css';
import { Layout, Row, Col, Button } from "antd";
import { WalletSelector } from '@aptos-labs/wallet-adapter-ant-design';
import "@aptos-labs/wallet-adapter-ant-design/dist/index.css";
import {
  AptosAccount,
  Provider,
  Network,
  CoinClient,
  TxnBuilderTypes,
  HexString,
} from "aptos";
import { Buffer } from 'buffer';
import { useWallet } from '@aptos-labs/wallet-adapter-react';

const provider = new Provider(Network.DEVNET);
//const provider = new Provider(Network.TESTNET);
const aptosClient = provider.aptosClient;
const coinClient = new CoinClient(provider.aptosClient);

const moduleAddress = "0xc3f4205e1f3d52fcd74385dfaabdd96edb6943a26db0ad8e6e7f7dd02b66ff85";
const moduleAddressArray = (new HexString(moduleAddress)).toUint8Array();
const eggCollectionName = "Egg Collection Name";

const privateKeyBytes_feePayer = Uint8Array.from(Buffer.from('f21423f436f7d44c2abd95b5a25323e81fc737040ab17ae8fe40dbf1b1de9e66', 'hex'));
const feePayer = new AptosAccount(privateKeyBytes_feePayer, '9bfdd4efe15f4d8aa145bef5f64588c7c391bcddaf34f9e977f59bd93b498f2a');

type FeePayerRawTransaction = TxnBuilderTypes.FeePayerRawTransaction;
type AccountAuthenticatorEd25519 = TxnBuilderTypes.AccountAuthenticatorEd25519;

async function getEggTokenAddr(ownerAddr: HexString): Promise<HexString | null> {
  const tokenOwnership = await provider.getOwnedTokens(ownerAddr);
  for (const ownership of tokenOwnership.current_token_ownerships_v2) {
    console.log(ownership.current_token_data?.current_collection?.collection_name);
    if(ownership.current_token_data?.current_collection?.collection_name === eggCollectionName){
      return new HexString(ownership.current_token_data.token_data_id);
    }
  }
  return null;
}


function equalUint8Array(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i=0; i<a.length; i++) {
        if (a[i] !== b[i]) return false;
    }
    return true;
}

async function signByFeePayer(feePayerTxn: FeePayerRawTransaction): Promise<AccountAuthenticatorEd25519 | null> {
  const feePayerTxnPayload = (feePayerTxn.raw_txn.payload as any).value;
  if (equalUint8Array(feePayerTxnPayload.module_name.address.address, moduleAddressArray) && feePayerTxnPayload.module_name.name.value === "egg") {
    return aptosClient.signMultiTransaction(feePayer, feePayerTxn);
  }
  return null;
}


function App() {
  //const { account, signAndSubmitTransaction, signMessage, signTransaction } = useWallet();
  const { account, signAndSubmitTransaction} = useWallet();
  const [balance, setBalance] = useState(NaN);
  const [feePayerBalance, setFeePayerBalance] = useState(NaN);
  const [deltaBalance, setDeltaBalance] = useState(0);
  const [deltaFeePayerBalance, setDeltaFeePayerBalance] = useState(0);
  const [counter, setCounter] = useState(0);

  const fetchBalance = async () => {
    if (!account) return [];
    const bal = Number(await coinClient.checkBalance(account?.address));
    setDeltaBalance(Number.isNaN(balance)?0:bal-balance);
    setBalance(bal);
    const feePayerBal = Number(await coinClient.checkBalance(feePayer.address()));
    setDeltaFeePayerBalance(Number.isNaN(feePayerBalance)?0:feePayerBal-feePayerBalance);
    setFeePayerBalance(feePayerBal);
  };
  useEffect(() => {
    fetchBalance();
  }, [account?.address, feePayer.address(), counter]);
  const mintEgg = async () => {
    if (!account) return;
    // build a transaction payload to be submitted
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::egg::mint_egg`,
      type_arguments: [],
      arguments: [],
    };
    // sign and submit transaction to chain
    const response = await signAndSubmitTransaction(payload);
    // wait for transaction
    await provider.waitForTransaction(response.hash);
    setCounter(counter + 1);
  };

  const mintEggFeePayer = async () => {
    if (!account) return;
    // build a transaction payload to be submitted
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::egg::mint_egg`,
      type_arguments: [],
      arguments: [],
    };
    const feePayerTxn = await aptosClient.generateFeePayerTransaction(account.address, payload, feePayer.address());
    // TODO: the following code is not working.
    //       `const senderAuthenticator = await aptosClient.signMultiTransaction(account, feePayerTxn);`
    const signature: string = await (window as any).petra.signMultiAgentTransaction(feePayerTxn);
    if (!signature) return;
    const signatureBytes = new HexString(signature).toUint8Array();
    const accountSignature = new TxnBuilderTypes.Ed25519Signature(signatureBytes);
    if (typeof account.publicKey !== 'string') {
      throw new Error('unexpected public key type');
    }
    const publicKeyBytes = new HexString(account.publicKey).toUint8Array();
    const senderAuthenticator = new TxnBuilderTypes.AccountAuthenticatorEd25519(
        new TxnBuilderTypes.Ed25519PublicKey(publicKeyBytes),
        accountSignature,
    );

    const feePayerAuthenticator = await signByFeePayer(feePayerTxn);
    if (!feePayerAuthenticator) return;
    // feePayer signs the transaction.
    // const feePayerTxnPayload = (feePayerTxn.raw_txn.payload as any).value;
    // console.log(equalUint8Array(feePayerTxnPayload.module_name.address.address, moduleAddressArray));
    // console.log(feePayerTxnPayload.module_name.name.value === "egg");
    // const feePayerAuthenticator = await aptosClient.signMultiTransaction(feePayer, feePayerTxn);

    // submit gas fee payer transaction
    const txn = await aptosClient.submitFeePayerTransaction(feePayerTxn, senderAuthenticator, feePayerAuthenticator);
    await aptosClient.waitForTransaction(txn.hash, { checkSuccess: true });
    setCounter(counter + 1);
  };

  const hatchEgg = async () => {
    // check for connected account
    if (!account) return;
    const eggTokenAddr = await getEggTokenAddr(new HexString(account.address));
    if (!eggTokenAddr) return;
    // build a transaction payload to be submitted
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::egg::hatch_egg`,
      type_arguments: [],
      arguments: [eggTokenAddr.hex()],
    };
    // sign and submit transaction to chain
    const response = await signAndSubmitTransaction(payload);
    // wait for transaction
    await provider.waitForTransaction(response.hash);
    setCounter(counter + 1);
  };

  const hatchEggFeePayer = async () => {
    // check for connected account
    if (!account) return;
    const eggTokenAddr = await getEggTokenAddr(new HexString(account.address));
    if (!eggTokenAddr) return;
    // build a transaction payload to be submitted
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::egg::hatch_egg`,
      type_arguments: [],
      arguments: [eggTokenAddr.hex()],
    };

    const feePayerTxn = await aptosClient.generateFeePayerTransaction(account.address, payload, feePayer.address());
    // TODO: the following code is not working.
    //       `const senderAuthenticator = await aptosClient.signMultiTransaction(account, feePayerTxn);`
    const signature: string = await (window as any).petra.signMultiAgentTransaction(feePayerTxn);
    if (!signature) return;
    const signatureBytes = new HexString(signature).toUint8Array();
    const accountSignature = new TxnBuilderTypes.Ed25519Signature(signatureBytes);
    if (typeof account.publicKey !== 'string') {
      throw new Error('unexpected public key type');
    }
    const publicKeyBytes = new HexString(account.publicKey).toUint8Array();
    const senderAuthenticator = new TxnBuilderTypes.AccountAuthenticatorEd25519(
      new TxnBuilderTypes.Ed25519PublicKey(publicKeyBytes),
      accountSignature,
    );

    const feePayerAuthenticator = await aptosClient.signMultiTransaction(feePayer, feePayerTxn);

    // submit gas fee payer transaction
    const txn = await aptosClient.submitFeePayerTransaction(feePayerTxn, senderAuthenticator, feePayerAuthenticator);
    await aptosClient.waitForTransaction(txn.hash, { checkSuccess: true });
    setCounter(counter + 1);
  };
  return (
    <>
      <Layout>
        <Row align="middle">
          <Col span={10} offset={2}>
            <h1>User client</h1>
          </Col>
          {/*<Col>*/}
          {/*  <h2>Balance: {balance}</h2>*/}
          {/*</Col>*/}
          <Col span={12} style={{ textAlign: "right", paddingRight: "200px" }}>
            <WalletSelector />
          </Col>
        </Row>
      </Layout>
      <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
        <Col span={20} offset={2}>
          {/*<h3> User account's balance: {balance} octas </h3>*/}
          {/*<h3> Fee payer's balance: {feePayerBalance} octas </h3>*/}
          <h2> User account's balance: {balance} octas <span style={{ color: (deltaBalance>0?"green":(deltaBalance===0?"black":"red"))}}> ({(deltaBalance > 0 ? "+" : "") + deltaBalance}) </span> </h2>
          <h2> Fee payer's balance: {feePayerBalance} octas <span style={{ color: (deltaFeePayerBalance>0?"green":(deltaFeePayerBalance===0?"black":"red"))}}> ({(deltaFeePayerBalance > 0 ? "+" : "") + deltaFeePayerBalance}) </span> </h2>
        </Col>
      </Row>
      <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
        <Col span={8} offset={2}>
          <Button onClick={mintEgg} block type="primary" style={{ fontSize:"large", height: "50px", backgroundColor: "#3f67ff" }}>
            Mint an egg token
          </Button>
        </Col>
        <Col span={9} offset={1}>
          <Button onClick={mintEggFeePayer} block type="primary" style={{ fontSize:"large", height: "50px", backgroundColor: "#008000" }}>
            Mint an egg token with fee payer
          </Button>
        </Col>
      </Row>
      <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
        <Col span={8} offset={2}>
          <Button onClick={hatchEgg} block type="primary" style={{ fontSize:"large", height: "50px", backgroundColor: "#3f67ff" }}>
            Hatch an egg token
          </Button>
        </Col>
        <Col span={9} offset={1}>
          <Button onClick={hatchEggFeePayer} block type="primary" style={{ fontSize:"large", height: "50px", backgroundColor: "#008000" }}>
            Hatch an egg token with fee payer
          </Button>
        </Col>
      </Row>
    </>
  );
}

export default App;
