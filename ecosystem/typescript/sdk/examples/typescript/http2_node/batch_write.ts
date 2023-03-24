import { AptosAccount } from "../../../dist";
import { Timer } from "timer-node";
import { Client } from "./client";
const { connect } = require("http2");
import { exit } from "process";

async function main() {
  const accountsCount = 50;
  const timer = new Timer();
  const client = new Client();

  timer.start();
  // create accounts
  const accounts: AptosAccount[] = [];
  const recipients: AptosAccount[] = [];
  for (let i = 0; i < accountsCount; i++) {
    accounts.push(new AptosAccount());
    recipients.push(new AptosAccount());
  }
  console.log(timer.time());

  // funds accounts
  const funds: Promise<string[]>[] = [];
  for (let i = 0; i < accounts.length; i++) {
    funds.push(client.fundAccount(accounts[i].address(), 100_000_000));
  }
  for (let i = 0; i < recipients.length; i++) {
    //funds.push(client.fundAccount(recipients[i].address(), 100_000_000));
  }
  await Promise.all(funds);

  console.log(timer.time());

  // read accounts
  const balances: Promise<any>[] = [];
  for (let i = 0; i < accounts.length; i++) {
    balances.push(client.getAccount(`accounts/${accounts[i].address().hex()}`));
  }

  await Promise.all(balances);

  console.log(timer.time());

  // initialize accounts with sequence number
  const accountSequenceNumbers: string[] = [];
  const sequenceNumbers: Promise<string[]>[] = [];
  for (let i = 0; i < accounts.length; i++) {
    let accountSequenceNumber = new AccountSequenceNumbers(client, accounts[i]);
  }

  // submit transactions
  // check for transactions

  exit(0);
}

class AccountSequenceNumbers {
  client: Client;
  account: AptosAccount;
  sequenceNumber: string;

  constructor(client: Client, acccount: AptosAccount) {
    this.client = client;
    this.account = acccount;
  }

  async getAccountCurrentSequenceNumber() {
    const { sequence_number } = await this.client.getAccount(this.account.address().hex());
    this.sequenceNumber = sequence_number;
  }
}

main();
