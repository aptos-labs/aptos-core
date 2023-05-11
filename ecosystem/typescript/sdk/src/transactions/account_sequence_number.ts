import { AptosAccount } from "../account";
import { Uint64 } from "../bcs";
import { Provider } from "../providers";
import { sleep } from "../utils";

const now = () => Math.floor(Date.now() / 1000);

export class AccountSequenceNumber {
  readonly provider: Provider;

  readonly account: AptosAccount;

  // sequence number on chain
  lastUncommintedNumber: Uint64 | null = null;

  // local sequence number
  currentNumber: Uint64 | null = null;

  lock = false;

  maximumInFlight = 100;

  sleepTime = 10;

  maxWaitTime = 30; // in seconds

  constructor(provider: Provider, account: AptosAccount) {
    this.provider = provider;
    this.account = account;
  }

  /**
   * Returns the next available sequnce number on this account
   *
   * @param block
   * @returns next available sequnce number
   */
  async nextSequenceNumber(block: boolean = true): Promise<bigint | null> {
    /*
    `lock` is used to prevent multiple coroutines from accessing a shared resource at the same time, 
    which can result in race conditions and data inconsistency.
    This implementation is not as robust as using a proper lock implementation 
    like `async-mutex` because it relies on busy waiting to acquire the lock, 
    which can be less efficient and may not work well in all scenarios
    */
    /* eslint-disable no-await-in-loop */
    while (this.lock) {
      await sleep(this.sleepTime);
    }

    this.lock = true;
    let nextNumber = BigInt(0);
    try {
      if (this.lastUncommintedNumber === null || this.currentNumber === null) {
        await this.initialize();
      }

      if (this.currentNumber! - this.lastUncommintedNumber! >= this.maximumInFlight) {
        await this.update();

        const startTime = now();
        while (this.currentNumber! - this.lastUncommintedNumber! >= this.maximumInFlight) {
          if (!block) {
            return null;
          }
          await sleep(this.sleepTime);
          if (now() - startTime > this.maxWaitTime) {
            /* eslint-disable no-console */
            console.warn(`Waited over 30 seconds for a transaction to commit, resyncing ${this.account.address()}`);
            await this.initialize();
          } else {
            await this.update();
          }
        }
      }
      nextNumber = this.currentNumber!;
      this.currentNumber! += BigInt(1);
    } catch (e) {
      console.error("error", e);
    } finally {
      this.lock = false;
    }
    return nextNumber;
  }

  /**
   * Initializes this account with the sequnce number on chain
   */
  async initialize(): Promise<void> {
    const { sequence_number: sequenceNumber } = await this.provider.getAccount(this.account.address());
    this.currentNumber = BigInt(sequenceNumber);
    this.lastUncommintedNumber = BigInt(sequenceNumber);
  }

  /**
   * Updates this account sequnce number with the one on-chain
   *
   * @returns on-chain sequnce number for this account
   */
  async update(): Promise<bigint> {
    const { sequence_number: sequenceNumber } = await this.provider.getAccount(this.account.address());
    this.lastUncommintedNumber = BigInt(sequenceNumber);
    return this.lastUncommintedNumber;
  }

  /**
   * Synchronizes local sequnce number with the sequnce number on chain for this account.
   *
   * Poll the network until all submitted transactions have either been committed or until
   * the maximum wait time has elapsed
   */
  async synchronize(): Promise<void> {
    if (this.lastUncommintedNumber === this.currentNumber) return;

    while (this.lock) {
      await sleep(this.sleepTime);
    }

    try {
      await this.update();
      const startTime = now();
      while (this.lastUncommintedNumber !== this.currentNumber) {
        if (now() - startTime > this.maxWaitTime) {
          /* eslint-disable no-console */
          console.warn(`Waited over 30 seconds for a transaction to commit, resyncing ${this.account.address()}`);
          await this.initialize();
        } else {
          await sleep(this.sleepTime);
          await this.update();
        }
      }
    } catch (e) {
      console.error("error", e);
    } finally {
      this.lock = false;
    }
  }
}
