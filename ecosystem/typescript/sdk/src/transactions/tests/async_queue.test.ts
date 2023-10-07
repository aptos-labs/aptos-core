import { AsyncQueue, AsyncQueueCancelledError } from "../async_queue";

describe("asyncQueue", () => {
  it("should enqueue and dequeue items", async () => {
    const asyncQueue = new AsyncQueue<number>();

    asyncQueue.enqueue(1);

    const item1 = await asyncQueue.dequeue();

    asyncQueue.enqueue(2);
    asyncQueue.enqueue(3);

    const item2 = await asyncQueue.dequeue();
    const item3 = await asyncQueue.dequeue();

    expect(item1).toBe(1);
    expect(item2).toBe(2);
    expect(item3).toBe(3);
  });

  it("should handle dequeue before queue", async () => {
    const asyncQueue = new AsyncQueue<number>();

    const itemPromise1 = asyncQueue.dequeue();
    const itemPromise2 = asyncQueue.dequeue();

    expect(asyncQueue.pendingDequeueLength()).toBe(2);

    asyncQueue.enqueue(1);
    asyncQueue.enqueue(2);

    const item1 = await itemPromise1;
    const item2 = await itemPromise2;

    expect(item1).toBe(1);
    expect(item2).toBe(2);

    expect(asyncQueue.pendingDequeueLength()).toBe(0);
  });

  it("should handle cancellation", async () => {
    const asyncQueue = new AsyncQueue<number>();

    const itemPromise1 = asyncQueue.dequeue();
    const itemPromise2 = asyncQueue.dequeue();

    expect(asyncQueue.pendingDequeueLength()).toBe(2);

    asyncQueue.cancel();

    await expect(itemPromise1).rejects.toThrow(AsyncQueueCancelledError);
    await expect(itemPromise2).rejects.toThrow(AsyncQueueCancelledError);

    expect(asyncQueue.isCancelled()).toBe(true);

    expect(asyncQueue.pendingDequeueLength()).toBe(0);
  });

  it("should handle cancellation without errors if queue is empty", () => {
    const asyncQueue = new AsyncQueue<number>();

    asyncQueue.cancel();

    expect(asyncQueue.isCancelled()).toBe(true);
  });

  it("should check if the queue is empty", () => {
    const asyncQueue = new AsyncQueue<number>();
    expect(asyncQueue.isEmpty()).toBe(true);

    asyncQueue.enqueue(1);

    expect(asyncQueue.isEmpty()).toBe(false);
  });

  it("should remove cancelled status after a new enqueue", () => {
    const asyncQueue = new AsyncQueue<number>();

    asyncQueue.cancel();

    expect(asyncQueue.isCancelled()).toBe(true);

    asyncQueue.enqueue(1);

    expect(asyncQueue.isCancelled()).toBe(false);
  });
});
