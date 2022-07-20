import { useState, useEffect } from 'react';

/**
 * Trigger a change event only if the value hasn't changed
 * for the specified amount of time
 */
export default function useDebounce<T>(value: T, delayMs: number = 500) {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const handler = setTimeout(() => setDebouncedValue(value), delayMs);
    // Cancel the timeout on unmount
    return () => clearTimeout(handler);
  }, [value, delayMs]);

  return debouncedValue;
}
