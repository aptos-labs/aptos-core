import { TransactionBuilderRemoteABI } from "../../transaction_builder";
import { AptosClient } from "../../providers";
import { getFaucetClient, longTestTimeout, NODE_URL } from "./test_helper.test";
import { AptosAccount } from "../../account";
import {
  RawTransaction,
  TransactionPayloadEntryFunction,
  TypeTagBool,
  TypeTagStruct,
  TypeTagU8,
  TypeTagVector,
} from "../../aptos_types";
import { HexString } from "../../utils";

describe.only("TransactionBuilderRemoteABI", () => {
  test(
    "generates raw txn from an entry function",
    async () => {
      const client = new AptosClient(NODE_URL);
      const alice = new AptosAccount();
      const faucetClient = getFaucetClient();
      await faucetClient.fundAccount(alice.address(), 100000000);
      // Create an instance of the class
      const builder = new TransactionBuilderRemoteABI(client, { sender: alice.address() });

      // Spy on the fetchABI method
      const fetchABISpy = jest.spyOn(builder, "fetchABI");

      // Mock the implementation of the fetchABI method to return a mock data
      const abi = new Map();
      abi.set("0x1::some_modules::SomeName", {
        fullName: "0x1::some_modules::SomeName",
        name: "SomeName",
        is_entry: true,
        is_view: false,
        generic_type_params: [],
        params: ["&signer", "0x1::string::String"],
        return: [],
        visibility: "public",
      });
      fetchABISpy.mockResolvedValue(abi);

      // Call the build method with some arguments
      const rawTxn = await builder.build("0x1::some_modules::SomeName", [], ["key"]);
      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.sender.address).toEqual(new HexString(alice.address().hex()).toUint8Array());
      expect(rawTxn.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.module_name.name.value).toBe("some_modules");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.function_name.value).toBe("SomeName");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args).toHaveLength(0);
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.args).toHaveLength(1);

      // Restore the original implementation of the fetch method
      fetchABISpy.mockRestore();
    },
    longTestTimeout,
  );

  test(
    "generates raw txn from an entry function with Object type",
    async () => {
      const client = new AptosClient(NODE_URL);
      const alice = new AptosAccount();
      const faucetClient = getFaucetClient();
      await faucetClient.fundAccount(alice.address(), 100000000);
      // Create an instance of the class
      const builder = new TransactionBuilderRemoteABI(client, { sender: alice.address() });

      // Spy on the fetchABI method
      const fetchABISpy = jest.spyOn(builder, "fetchABI");

      // Mock the implementation of the fetchABI method to return a mock data
      const abi = new Map();
      abi.set("0x1::some_modules::SomeName", {
        fullName: "0x1::some_modules::SomeName",
        name: "SomeName",
        is_entry: true,
        is_view: false,
        generic_type_params: [
          {
            constraints: ["key"],
          },
        ],
        params: ["&signer", "0x1::object::Object<T>", "0x1::string::String"],
        return: [],
        visibility: "public",
      });
      fetchABISpy.mockResolvedValue(abi);

      // Call the build method with some arguments
      const rawTxn = await builder.build(
        "0x1::some_modules::SomeName",
        ["0x1::type::SomeType"],
        ["0x2b4d540735a4e128fda896f988415910a45cab41c9ddd802b32dd16e8f9ca3cd", "key"],
      );

      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.sender.address).toEqual(new HexString(alice.address().hex()).toUint8Array());
      expect(rawTxn.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.module_name.name.value).toBe("some_modules");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.function_name.value).toBe("SomeName");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args).toHaveLength(1);
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.args).toHaveLength(2);

      // Restore the original implementation of the fetch method
      fetchABISpy.mockRestore();
    },
    longTestTimeout,
  );

  test(
    "generates raw txn from a generic entry function",
    async () => {
      const client = new AptosClient(NODE_URL);
      const alice = new AptosAccount();
      const faucetClient = getFaucetClient();
      await faucetClient.fundAccount(alice.address(), 100000000);
      // Create an instance of the class
      const builder = new TransactionBuilderRemoteABI(client, { sender: alice.address() });

      // Spy on the fetchABI method
      const fetchABISpy = jest.spyOn(builder, "fetchABI");

      // Mock the implementation of the fetchABI method to return a mock data
      const abi = new Map();
      abi.set("0x1::some_modules::SomeName", {
        fullName: "0x1::some_modules::SomeName",
        name: "SomeName",
        is_entry: true,
        is_view: false,
        generic_type_params: [
          {
            constraints: ["key"],
          },
          {
            constraints: ["drop"],
          },
        ],
        params: ["&signer", "0x1::object::Object<T>", "0x1::string::String", "T1"],
        return: [],
        visibility: "public",
      });
      fetchABISpy.mockResolvedValue(abi);

      // Call the build method with some arguments
      const rawTxn = await builder.build(
        "0x1::some_modules::SomeName",
        ["0x1::type::SomeType", "vector<u8>"],
        ["0x2b4d540735a4e128fda896f988415910a45cab41c9ddd802b32dd16e8f9ca3cd", "key", "[hello,world]"],
      );

      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.sender.address).toEqual(new HexString(alice.address().hex()).toUint8Array());
      expect(rawTxn.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.module_name.name.value).toBe("some_modules");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.function_name.value).toBe("SomeName");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args).toHaveLength(2);
      expect(
        (rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args[0] instanceof TypeTagStruct,
      ).toBeTruthy();
      expect(
        (rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args[1] instanceof TypeTagVector,
      ).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.args).toHaveLength(3);

      // Restore the original implementation of the fetch method
      fetchABISpy.mockRestore();
    },
    longTestTimeout,
  );

  test(
    "generates raw txn from an entry function with multiple generic params",
    async () => {
      const client = new AptosClient(NODE_URL);
      const alice = new AptosAccount();
      const faucetClient = getFaucetClient();
      await faucetClient.fundAccount(alice.address(), 100000000);
      // Create an instance of the class
      const builder = new TransactionBuilderRemoteABI(client, { sender: alice.address() });

      // Spy on the fetchABI method
      const fetchABISpy = jest.spyOn(builder, "fetchABI");

      // Mock the implementation of the fetchABI method to return a mock data
      const abi = new Map();
      abi.set("0x1::some_modules::SomeName", {
        fullName: "0x1::some_modules::SomeName",
        name: "SomeName",
        is_entry: true,
        is_view: false,
        generic_type_params: [
          {
            constraints: ["key"],
          },
          {
            constraints: ["drop"],
          },
          {
            constraints: ["key"],
          },
        ],
        params: ["&signer", "T0", "0x1::string::String", "T1", "T2"],
        return: [],
        visibility: "public",
      });
      fetchABISpy.mockResolvedValue(abi);

      // Call the build method with some arguments
      const rawTxn = await builder.build(
        "0x1::some_modules::SomeName",
        ["bool", "vector<u8>", "u8"],
        ["true", "key", "[hello,world]", "6"],
      );

      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.sender.address).toEqual(new HexString(alice.address().hex()).toUint8Array());
      expect(rawTxn.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.module_name.name.value).toBe("some_modules");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.function_name.value).toBe("SomeName");
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args).toHaveLength(3);
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args[0] instanceof TypeTagBool).toBeTruthy();
      expect(
        (rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args[1] instanceof TypeTagVector,
      ).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.ty_args[2] instanceof TypeTagU8).toBeTruthy();
      expect((rawTxn.payload as TransactionPayloadEntryFunction).value.args).toHaveLength(4);

      // Restore the original implementation of the fetch method
      fetchABISpy.mockRestore();
    },
    longTestTimeout,
  );
});
