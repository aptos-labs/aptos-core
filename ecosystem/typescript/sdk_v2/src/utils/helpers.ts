import { AptosApiError } from "../client/types";

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly message: string,
    public readonly errorCode?: string,
    public readonly vmErrorCode?: string,
  ) {
    super(message);
  }
}

export function parseApiError(target: unknown, propertyKey: string, descriptor: PropertyDescriptor) {
  const childFunction = descriptor.value;
  // eslint-disable-next-line no-param-reassign
  descriptor.value = async function wrapper(...args: any[]) {
    try {
      // We need to explicitly await here so that the function is called and
      // potentially throws an error. If we just return without awaiting, the
      // promise is returned directly and the catch block cannot trigger.
      const res = await childFunction.apply(this, [...args]);
      return res;
    } catch (e) {
      if (e instanceof AptosApiError) {
        throw new ApiError(
          e.status,
          JSON.stringify({ message: e.message, ...e.data }),
          e.data?.error_code,
          e.data?.vm_error_code,
        );
      }
      throw e;
    }
  };
  return descriptor;
}
