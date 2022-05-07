import { AptosClient, raiseForStatus } from "./aptos_client";
import { AnyObject } from "./util";
import { AxiosResponse } from "axios";

import { NODE_URL } from "./util.test";

test("gets genesis account", async () => {
  const client = new AptosClient(NODE_URL);
  const account = await client.getAccount("0x1");
  expect(account.authentication_key.length).toBe(66);
  expect(account.sequence_number).not.toBeNull();
});

test("gets transactions", async () => {
  const client = new AptosClient(NODE_URL);
  const transactions = await client.getTransactions();
  expect(transactions.length).toBeGreaterThan(0);
});

test("gets genesis resources", async () => {
  const client = new AptosClient(NODE_URL);
  const resources = await client.getAccountResources("0x1");
  const accountResource = resources.find((r) => r.type === "0x1::Account::Account");
  expect((accountResource.data as AnyObject)["self_address"]).toBe("0x1");
});

test("gets the Account resource", async () => {
  const client = new AptosClient(NODE_URL);
  const accountResource = await client.getAccountResource("0x1", "0x1::Account::Account");
  expect((accountResource.data as AnyObject)["self_address"]).toBe("0x1");
});

test("gets ledger info", async () => {
  const client = new AptosClient(NODE_URL);
  const ledgerInfo = await client.getLedgerInfo();
  expect(ledgerInfo.chain_id).toBeGreaterThan(1);
  expect(parseInt(ledgerInfo.ledger_version, 10)).toBeGreaterThan(0);
});

test("gets account modules", async () => {
  const client = new AptosClient(NODE_URL);
  const modules = await client.getAccountModules("0x1");
  const module = modules.find((r) => r.abi.name === "TestCoin");
  expect(module.abi.address).toBe("0x1");
});

test("gets the TestCoin module", async () => {
  const client = new AptosClient(NODE_URL);
  const module = await client.getAccountModule("0x1", "TestCoin");
  expect(module.abi.address).toBe("0x1");
});

test("test raiseForStatus", async () => {
  const testData = { hello: "wow" };
  const fakeResponse: AxiosResponse = {
    status: 200,
    statusText: "Status Text",
    data: "some string",
    request: {
      host: "host",
      path: "/path",
    },
  } as AxiosResponse;

  // Shouldn't throw
  raiseForStatus(200, fakeResponse, testData);
  raiseForStatus(200, fakeResponse);

  // an error, oh no!
  fakeResponse.status = 500;
  expect(() => raiseForStatus(200, fakeResponse, testData)).toThrow(
    'Status Text - "some string" @ host/path : {"hello":"wow"}',
  );

  expect(() => raiseForStatus(200, fakeResponse)).toThrow('Status Text - "some string" @ host/path');

  // Just a wild test to make sure it doesn't break: request is `any`!
  delete fakeResponse.request;
  expect(() => raiseForStatus(200, fakeResponse, testData)).toThrow('Status Text - "some string" : {"hello":"wow"}');

  expect(() => raiseForStatus(200, fakeResponse)).toThrow('Status Text - "some string"');
});
