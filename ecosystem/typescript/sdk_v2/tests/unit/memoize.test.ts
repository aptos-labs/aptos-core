import { memoizeAsync, memoize } from "../../src/utils/memoize";

describe("memoize", () => {
  describe("memoizeAsync", () => {
    // Define an asynchronous function to mock
    const asyncFunction = jest.fn(async (arg) => {
      // Simulate some asynchronous operation to be resolved after 100 ms
      await new Promise((resolve) => setTimeout(resolve, 100));
      return arg;
    });

    beforeEach(() => {
      // needed because of a weird bug with jest https://github.com/jestjs/jest/pull/12572
      jest.useFakeTimers({ doNotFake: ["setTimeout", "performance"] });
    });

    afterEach(() => {
      // Restore real timers and clear all timers after each test
      jest.useRealTimers();
      jest.clearAllTimers();
      asyncFunction.mockClear();
    });

    test("it does not execute function again before TTL has passed", async () => {
      // Create a memoized version of the async function with a TTL of 200 milliseconds
      const memoizedAsyncFunction = memoizeAsync(asyncFunction, "asyncFunction1", 200);

      // Call the memoized function with an argument
      const result1 = await memoizedAsyncFunction("arg1");

      // Advance the timers by 50 milliseconds (before the TTL)
      jest.advanceTimersByTime(50);

      // Call the memoized function again with the same argument
      const result2 = await memoizedAsyncFunction("arg2");

      // Ensure the function was not executed again before TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg1"); // Result from the memoized cache

      // Ensure the async function was called only once
      expect(asyncFunction).toHaveBeenCalledTimes(1);
    });

    test("it executes function again after TTL has passed", async () => {
      // Create a memoized version of the async function with a TTL of 200 milliseconds
      const memoizedAsyncFunction = memoizeAsync(asyncFunction, "asyncFunction2", 200);

      // Call the memoized function with an argument
      const result1 = await memoizedAsyncFunction("arg1");

      // Advance the timers by 250 milliseconds (beyond the TTL)
      jest.advanceTimersByTime(250);

      // Call the memoized function again with the same argument
      const result2 = await memoizedAsyncFunction("arg2");

      // Ensure the function was executed again after TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg2"); // Result from the second call

      // Ensure the async function was called twice (once initially, once after TTL)
      expect(asyncFunction).toHaveBeenCalledTimes(2);
    });

    test("it does not execute function again when TTL is not provided", async () => {
      const memoizedAsyncFunction = memoizeAsync(asyncFunction, "asyncFunction3");

      // Call the memoized function with an argument
      const result1 = await memoizedAsyncFunction("arg1");

      // Advance the timers by 250 milliseconds (beyond the TTL)
      jest.advanceTimersByTime(250);

      // Call the memoized function again with the same argument
      const result2 = await memoizedAsyncFunction("arg2");

      // Ensure the function was executed again after TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg1"); // Result from the memoized cache

      // Ensure the async function was called only once
      expect(asyncFunction).toHaveBeenCalledTimes(1);
    });

    test("it returns the expected response based on the provided cache key", async () => {
      const memoizedAsyncFunction4 = memoizeAsync(asyncFunction, "asyncFunction4");
      const memoizedAsyncFunction5 = memoizeAsync(asyncFunction, "asyncFunction5");

      // Call memoized function with an argument
      const result1 = await memoizedAsyncFunction4("arg1");

      // Call another memoized function with an argument
      const result2 = await memoizedAsyncFunction5("arg2");

      // Ensure the function was executed twice
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg2"); // Result from the second call

      // Ensure the async function was called twice (once for each cache key)
      expect(asyncFunction).toHaveBeenCalledTimes(2);
    });
  });

  describe("memoize", () => {
    // Define a function to mock
    const func = jest.fn((arg) => {
      return arg;
    });

    beforeEach(() => {
      // needed because of a weird bug with jest https://github.com/jestjs/jest/pull/12572
      jest.useFakeTimers({ doNotFake: ["performance"] });
    });

    afterEach(() => {
      // Restore real timers and clear all timers after each test
      jest.useRealTimers();
      jest.clearAllTimers();
      func.mockClear();
    });

    test("it does not execute function again before TTL has passed", () => {
      // Create a memoized version of the async function with a TTL of 200 milliseconds
      const memoizedFunction = memoize(func, "function1", 200);

      // Call the memoized function with an argument
      const result1 = memoizedFunction("arg1");

      // Advance the timers by 50 milliseconds (before the TTL)
      jest.advanceTimersByTime(50);

      // Call the memoized function again with the same argument
      const result2 = memoizedFunction("arg2");

      // Ensure the function was not executed again before TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg1"); // Result from the memoized cache

      // Ensure the async function was called only once
      expect(func).toHaveBeenCalledTimes(1);
    });

    test("it executes function again after TTL has passed", () => {
      // Create a memoized version of the async function with a TTL of 200 milliseconds
      const memoizedFunction = memoize(func, "function2", 200);

      // Call the memoized function with an argument
      const result1 = memoizedFunction("arg1");

      // Advance the timers by 250 milliseconds (beyond the TTL)
      jest.advanceTimersByTime(250);

      // Call the memoized function again with the same argument
      const result2 = memoizedFunction("arg2");

      // Ensure the function was executed again after TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg2"); // Result from the second call

      // Ensure the async function was called twice (once initially, once after TTL)
      expect(func).toHaveBeenCalledTimes(2);
    });

    test("it does not execute function again when TTL is not provided", () => {
      const memoizedFunction = memoize(func, "function3");

      // Call the memoized function with an argument
      const result1 = memoizedFunction("arg1");

      // Advance the timers by 250 milliseconds (beyond the TTL)
      jest.advanceTimersByTime(250);

      // Call the memoized function again with the same argument
      const result2 = memoizedFunction("arg2");

      // Ensure the function was executed again after TTL
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg1"); // Result from the memoized cache

      // Ensure the async function was called only once
      expect(func).toHaveBeenCalledTimes(1);
    });

    test("it returns the expected response based on the provided cache key", () => {
      const memoizedFunction4 = memoize(func, "function4");
      const memoizedFunction5 = memoize(func, "function5");

      // Call memoized function with an argument
      const result1 = memoizedFunction4("arg1");

      // Call another memoized function with an argument
      const result2 = memoizedFunction5("arg2");

      // Ensure the function was executed twice
      expect(result1).toBe("arg1"); // Result from the first call
      expect(result2).toBe("arg2"); // Result from the second call

      // Ensure the async function was called twice (once for each cache key)
      expect(func).toHaveBeenCalledTimes(2);
    });
  });
});
