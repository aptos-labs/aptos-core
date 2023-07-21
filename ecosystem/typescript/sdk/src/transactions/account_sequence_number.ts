/**
 * A wrapper that handles and manages an account sequence number.
 *
 * Submit up to `maximumInFlight` transactions per account in parallel with a timeout of `sleepTime`
 * If local assumes `maximumInFlight` are in flight, determine the actual committed state from the network
 * If there are less than `maximumInFlight` due to some being committed, adjust the window
 * If `maximumInFlight` are in flight, wait `sleepTime` seconds before re-evaluating
 * If ever waiting more than `maxWaitTime` restart the sequence number to the current on-chain state
 *
 * Assumptions:
 * Accounts are expected to be managed by a single AccountSequenceNumber and not used otherwise.
 * They are initialized to the current on-chain state, so if there are already transactions in
 * flight, they may take some time to reset.
 * Accounts are automatically initialized if not explicitly
 *
 * Notes:
 * This is co-routine safe, that is many async tasks can be reading from this concurrently.
 * The state of an account cannot be used across multiple AccountSequenceNumber services.
 * The synchronize method will create a barrier that prevents additional nextSequenceNumber
 * calls until it is complete.
 * This only manages the distribution of sequence numbers it does not help handle transaction
 * failures.
 * If a transaction fails, you should call synchronize and wait for timeouts.
 */

import { AptosAccount } from "../account";
import { Provider } from "../providers";
import { sleep } from "../utils";

// returns `now` time in seconds
const now = () => Math.floor(Date.now() / 1000);

export class AccountSequenceNumber {
  readonly provider: Provider;

  readonly account: AptosAccount;

  // sequence number on chain
  lastUncommintedNumber: bigint | null = null;

  // local sequence number
  currentNumber: bigint | null = null;

  /**
   * We want to guarantee that we preserve ordering of workers to requests.
   *
   * `lock` is used to try to prevent multiple coroutines from accessing a shared resource at the same time,
   * which can result in race conditions and data inconsistency.
   * This code actually doesn't do it though, since we aren't giving out a slot, it is still somewhat a race condition.
   *
   * The ideal solution is likely that each thread grabs the next number from a incremental integer.
   * When they complete, they increment that number and that entity is able to enter the `lock`.
   * That would guarantee ordering.
   */
  lock = false;

  maxWaitTime: number;

  maximumInFlight: number;

  sleepTime: number;

  constructor(
    provider: Provider,
    account: AptosAccount,
    maxWaitTime: number,
    maximumInFlight: number,
    sleepTime: number,
  ) {
    this.provider = provider;
    this.account = account;
    this.maxWaitTime = maxWaitTime;
    this.maximumInFlight = maximumInFlight;
    this.sleepTime = sleepTime;
  }

  /**
   * Returns the next available sequence number for this account
   *
   * @returns next available sequence number
   */
  async nextSequenceNumber(): Promise<bigint | null> {
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
      console.error("error in getting next sequence number for this account", e);
    } finally {
      this.lock = false;
    }
    return nextNumber;
  }

  /**
   * Initializes this account with the sequence number on chain
   */
  async initialize(): Promise<void> {
    const { sequence_number: sequenceNumber } = await this.provider.getAccount(this.account.address());
    this.currentNumber = BigInt(sequenceNumber);
    this.lastUncommintedNumber = BigInt(sequenceNumber);
  }

  /**
   * Updates this account sequence number with the one on-chain
   *
   * @returns on-chain sequence number for this account
   */
  async update(): Promise<bigint> {
    const { sequence_number: sequenceNumber } = await this.provider.getAccount(this.account.address());
    this.lastUncommintedNumber = BigInt(sequenceNumber);
    return this.lastUncommintedNumber;
  }

  /**
   * Synchronizes local sequence number with the seqeunce number on chain for this account.
   *
   * Poll the network until all submitted transactions have either been committed or until
   * the maximum wait time has elapsed
   */
  async synchronize(): Promise<void> {
    if (this.lastUncommintedNumber === this.currentNumber) return;

    /* eslint-disable no-await-in-loop */
    while (this.lock) {
      await sleep(this.sleepTime);
    }

    this.lock = true;

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
      console.error("error in synchronizing this account sequence number with the one on chain", e);
    } finally {
      this.lock = false;
    }
  }
}
