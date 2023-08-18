import { AptosAccount } from "../../account";
import { Provider } from "../../providers";
import { AccountSequenceNumber } from "../account_sequence_number";
import { getFaucetClient, longTestTimeout, PROVIDER_LOCAL_NETWORK_CONFIG } from "../../tests/unit/test_helper.test";

const provider = new Provider(PROVIDER_LOCAL_NETWORK_CONFIG);
const account = new AptosAccount();
const faucet = getFaucetClient();
const accountSequenceNumber = new AccountSequenceNumber(provider, account, 30, 100, 10);
let getAccountSpy: jest.SpyInstance;

let lastSeqNumber: bigint | null;

describe("account sequence number", () => {
  beforeAll(async () => {
    await faucet.fundAccount(account.address(), 1000000);
  }, longTestTimeout);

  beforeEach(() => {
    getAccountSpy = jest.spyOn(provider, "getAccount");
  });

  afterEach(() => {
    getAccountSpy.mockRestore();
  });

  it(
    "initializes with correct sequence number",
    async () => {
      await accountSequenceNumber.initialize();
      expect(accountSequenceNumber.currentNumber).toEqual(BigInt(0));
      expect(accountSequenceNumber.lastUncommintedNumber).toEqual(BigInt(0));
    },
    longTestTimeout,
  );

  it("updates with correct sequence number", async () => {
    const seqNum = "2";
    getAccountSpy.mockResolvedValue({
      sequence_number: seqNum,
      authentication_key: account.authKey().hex(),
    });
    await accountSequenceNumber.update();
    expect(accountSequenceNumber.lastUncommintedNumber).toEqual(BigInt(parseInt(seqNum)));
  });

  it(
    "returns sequential number starting from 0",
    async () => {
      getAccountSpy.mockResolvedValue({
        sequence_number: "0",
        authentication_key: account.authKey().hex(),
      });
      for (let seqNum = 0; seqNum < 5; seqNum++) {
        lastSeqNumber = await accountSequenceNumber.nextSequenceNumber();
        expect(lastSeqNumber).toEqual(BigInt(seqNum));
      }
    },
    longTestTimeout,
  );

  it(
    "includes updated on-chain sequnce number in local sequence number",
    async () => {
      const previousSeqNum = "5";
      getAccountSpy.mockResolvedValue({
        sequence_number: previousSeqNum,
        authentication_key: account.authKey().hex(),
      });
      for (let seqNum = 0; seqNum < accountSequenceNumber.maximumInFlight; seqNum++) {
        lastSeqNumber = await accountSequenceNumber.nextSequenceNumber();
        expect(lastSeqNumber).toEqual(BigInt(seqNum + parseInt(previousSeqNum)));
      }
    },
    longTestTimeout,
  );

  it(
    "synchronize completes when local and on-chain sequnec number equal",
    async () => {
      const nextSequenceNumber = lastSeqNumber! + BigInt(1);

      getAccountSpy.mockResolvedValue({
        sequence_number: nextSequenceNumber + "",
        authentication_key: account.authKey().hex(),
      });

      expect(accountSequenceNumber.currentNumber).not.toEqual(lastSeqNumber);
      await accountSequenceNumber.synchronize();
      expect(accountSequenceNumber.currentNumber).toEqual(nextSequenceNumber);
    },
    longTestTimeout,
  );
});
