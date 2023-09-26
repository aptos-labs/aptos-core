/**
 * The global cache Map
 */
const cache = new Map<string, { value: any; timestamp: number }>();

/**
 * A memoize high order function to cache async function response
 *
 * @param func An async function to cache the result of
 * @param key The provided cache key
 * @param ttlMs time-to-live in milliseconds for cached data
 * @returns
 */
export function memoizeAsync<T>(
  func: (...args: any[]) => Promise<T>,
  key: string,
  ttlMs?: number,
): (...args: any[]) => Promise<T> {
  return async (...args: any[]) => {
    // Check if the cached result exists and is within TTL
    if (cache.has(key)) {
      const { value, timestamp } = cache.get(key)!;
      if (ttlMs === undefined || Date.now() - timestamp <= ttlMs) {
        return value;
      }
    }

    // If not cached or TTL expired, compute the result
    const result = await func(...args);

    // Cache the result with a timestamp
    cache.set(key, { value: result, timestamp: Date.now() });

    return result;
  };
}

/**
 * A memoize high order function to cache function response
 *
 * @param func A function to cache the result of
 * @param key The provided cache key
 * @param ttlMs time-to-live in milliseconds for cached data
 * @returns
 */
export function memoize<T>(func: (...args: any[]) => T, key: string, ttlMs?: number): (...args: any[]) => T {
  return (...args: any[]) => {
    // Check if the cached result exists and is within TTL
    if (cache.has(key)) {
      const { value, timestamp } = cache.get(key)!;
      if (ttlMs === undefined || Date.now() - timestamp <= ttlMs) {
        return value;
      }
    }

    // If not cached or TTL expired, compute the result
    const result = func(...args);

    // Cache the result with a timestamp
    cache.set(key, { value: result, timestamp: Date.now() });

    return result;
  };
}
