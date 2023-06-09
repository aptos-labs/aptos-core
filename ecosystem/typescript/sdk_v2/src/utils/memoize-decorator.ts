/**
 * Credits to https://github.com/darrylhodgins/typescript-memoize
 */

/* eslint-disable no-param-reassign */
/* eslint-disable no-restricted-syntax */

interface MemoizeArgs {
  // ttl in milliseconds for cached items. After `ttlMs`, cached items are evicted automatically. If no `ttlMs`
  // is provided, cached items won't get auto-evicted.
  ttlMs?: number;
  // produces the cache key based on `args`.
  hashFunction?: boolean | ((...args: any[]) => any);
  // cached items can be taged with `tags`. `tags` can be used to evict cached items
  tags?: string[];
}

export function Memoize(args?: MemoizeArgs | MemoizeArgs["hashFunction"]) {
  let hashFunction: MemoizeArgs["hashFunction"];
  let ttlMs: MemoizeArgs["ttlMs"];
  let tags: MemoizeArgs["tags"];

  if (typeof args === "object") {
    hashFunction = args.hashFunction;
    ttlMs = args.ttlMs;
    tags = args.tags;
  } else {
    hashFunction = args;
  }

  return (target: Object, propertyKey: string, descriptor: TypedPropertyDescriptor<any>) => {
    if (descriptor.value != null) {
      descriptor.value = getNewFunction(descriptor.value, hashFunction, ttlMs, tags);
    } else if (descriptor.get != null) {
      descriptor.get = getNewFunction(descriptor.get, hashFunction, ttlMs, tags);
    } else {
      throw new Error("Only put a Memoize() decorator on a method or get accessor.");
    }
  };
}

export function MemoizeExpiring(ttlMs: number, hashFunction?: MemoizeArgs["hashFunction"]) {
  return Memoize({
    ttlMs,
    hashFunction,
  });
}

const clearCacheTagsMap: Map<string, Map<any, any>[]> = new Map();

export function clear(tags: string[]): number {
  const cleared: Set<Map<any, any>> = new Set();
  for (const tag of tags) {
    const maps = clearCacheTagsMap.get(tag);
    if (maps) {
      for (const mp of maps) {
        if (!cleared.has(mp)) {
          mp.clear();
          cleared.add(mp);
        }
      }
    }
  }
  return cleared.size;
}

function getNewFunction(
  originalMethod: () => void,
  hashFunction?: MemoizeArgs["hashFunction"],
  ttlMs: number = 0,
  tags?: MemoizeArgs["tags"],
) {
  const propMapName = Symbol("__memoized_map__");

  // The function returned here gets called instead of originalMethod.
  // eslint-disable-next-line func-names
  return function (...args: any[]) {
    let returnedValue: any;

    // @ts-ignore
    const that: any = this;

    // Get or create map
    // eslint-disable-next-line no-prototype-builtins
    if (!that.hasOwnProperty(propMapName)) {
      Object.defineProperty(that, propMapName, {
        configurable: false,
        enumerable: false,
        writable: false,
        value: new Map<any, any>(),
      });
    }
    const myMap: Map<any, any> = that[propMapName];

    if (Array.isArray(tags)) {
      for (const tag of tags) {
        if (clearCacheTagsMap.has(tag)) {
          clearCacheTagsMap.get(tag)!.push(myMap);
        } else {
          clearCacheTagsMap.set(tag, [myMap]);
        }
      }
    }

    if (hashFunction || args.length > 0 || ttlMs > 0) {
      let hashKey: any;

      // If true is passed as first parameter, will automatically use every argument, passed to string
      if (hashFunction === true) {
        hashKey = args.map((a) => a.toString()).join("!");
      } else if (hashFunction) {
        hashKey = hashFunction.apply(that, args);
      } else {
        // eslint-disable-next-line prefer-destructuring
        hashKey = args[0];
      }

      const timestampKey = `${hashKey}__timestamp`;
      let isExpired: boolean = false;
      if (ttlMs > 0) {
        if (!myMap.has(timestampKey)) {
          // "Expired" since it was never called before
          isExpired = true;
        } else {
          const timestamp = myMap.get(timestampKey);
          isExpired = Date.now() - timestamp > ttlMs;
        }
      }

      if (myMap.has(hashKey) && !isExpired) {
        returnedValue = myMap.get(hashKey);
      } else {
        returnedValue = originalMethod.apply(that, args as any);
        myMap.set(hashKey, returnedValue);
        if (ttlMs > 0) {
          myMap.set(timestampKey, Date.now());
        }
      }
    } else {
      const hashKey = that;
      if (myMap.has(hashKey)) {
        returnedValue = myMap.get(hashKey);
      } else {
        returnedValue = originalMethod.apply(that, args as any);
        myMap.set(hashKey, returnedValue);
      }
    }

    return returnedValue;
  };
}
