/**
 * The AsyncQueue class is an async-aware data structure that provides a queue-like
 * behavior for managing asynchronous tasks or operations.
 * It allows to enqueue items and dequeue them asynchronously.
 * This is not thread-safe but it is async concurrency safe and
 * it does not guarantee ordering for those that call into and await on enqueue.
 */

interface PendingDequeue<T> {
  resolve: (value: T) => void;
  reject: (reason?: AsyncQueueCancelledError) => void;
}

export class AsyncQueue<T> {
  readonly queue: T[] = [];

  // The pendingDequeue is used to handle the resolution of promises when items are enqueued and dequeued.
  private pendingDequeue: PendingDequeue<T>[] = [];

  private cancelled: boolean = false;

  /**
   * The enqueue method adds an item to the queue. If there are pending dequeued promises,
   * in the pendingDequeue, it resolves the oldest promise with the enqueued item immediately.
   * Otherwise, it adds the item to the queue.
   *
   * @param item T
   */
  enqueue(item: T): void {
    this.cancelled = false;

    if (this.pendingDequeue.length > 0) {
      const promise = this.pendingDequeue.shift();

      promise?.resolve(item);

      return;
    }

    this.queue.push(item);
  }

  /**
   * The dequeue method returns a promise that resolves to the next item in the queue.
   * If the queue is not empty, it resolves the promise immediately with the next item.
   * Otherwise, it creates a new promise. The promise's resolve function is stored
   * in the pendingDequeue with a unique counter value as the key.
   * The newly created promise is then returned, and it will be resolved later when an item is enqueued.
   *
   * @returns Promise<T>
   */
  async dequeue(): Promise<T> {
    if (this.queue.length > 0) {
      return Promise.resolve(this.queue.shift()!);
    }

    return new Promise<T>((resolve, reject) => {
      this.pendingDequeue.push({ resolve, reject });
    });
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

    this.pendingDequeue.forEach(async ({ reject }) => {
      reject(new AsyncQueueCancelledError("Task cancelled"));
    });

    this.pendingDequeue = [];

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

  /**
   * The pendingDequeueLength method returns the length of the pendingDequeue.
   *
   * @returns number
   */
  pendingDequeueLength(): number {
    return this.pendingDequeue.length;
  }
}

export class AsyncQueueCancelledError extends Error {}
