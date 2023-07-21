/**
 * The AsyncQueue class is an async-aware data structure that provides a queue-like
 * behavior for managing asynchronous tasks or operations.
 * It allows to enqueue items and dequeue them asynchronously.
 * This is not thread-safe but it is async concurrency safe and
 * it does not guarantee ordering for those that call into and await on enqueue.
 */

export class AsyncQueue<T> {
  readonly queue: T[] = [];

  // The resolveMap is used to handle the resolution of promises when items are enqueued and dequeued.
  private resolveMap: Map<number, (value: T) => void> = new Map();

  private counter: number = 0;

  private cancelled: boolean = false;

  /**
   * The enqueue method adds an item to the queue. If there are pending dequeued promises,
   * in the resolveMap, it resolves the oldest promise with the enqueued item immediately.
   * Otherwise, it adds the item to the queue.
   *
   * @param item T
   */
  enqueue(item: T): void {
    if (this.resolveMap.size > 0) {
      const resolve = this.resolveMap.get(0);
      if (resolve) {
        this.resolveMap.delete(0);
        resolve(item);
        return;
      }
    }
    this.queue.push(item);
  }

  /**
   * The dequeue method returns a promise that resolves to the next item in the queue.
   * If the queue is not empty, it resolves the promise immediately with the next item.
   * Otherwise, it creates a new promise. The promise's resolve function is stored
   * in the resolveMap with a unique counter value as the key.
   * The newly created promise is then returned, and it will be resolved later when an item is enqueued.
   *
   * @returns Promise<T>
   */
  async dequeue(): Promise<T> {
    if (this.queue.length > 0) {
      return Promise.resolve(this.queue.shift()!);
    }
    const promise = new Promise<T>((resolve) => {
      this.counter += 1;
      this.resolveMap.set(this.counter, resolve);
    });
    return promise;
  }

  /**
   * The isEmpty method returns whether the queue is empty or not.
   *
   * @returns boolean
   */
  isEmpty(): boolean {
    return this.queue.length === 0;
  }

  /**
   * The cancel method cancels all pending promises in the queue.
   * It rejects the promises with a AsyncQueueCancelledError error,
   * ensuring that any awaiting code can handle the cancellation appropriately.
   */
  cancel(): void {
    this.cancelled = true;
    this.resolveMap.forEach(async (resolve) => {
      resolve(await Promise.reject(new AsyncQueueCancelledError("Task cancelled")));
    });
    this.resolveMap.clear();
    this.queue.length = 0;
  }

  /**
   * The isCancelled method returns whether the queue is cancelled or not.
   *
   * @returns boolean
   */
  isCancelled(): boolean {
    return this.cancelled;
  }
}

export class AsyncQueueCancelledError extends Error {
  /* eslint-disable @typescript-eslint/no-useless-constructor */
  constructor(message: string) {
    super(message);
  }
}
