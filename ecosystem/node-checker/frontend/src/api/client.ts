export enum ResponseErrorType {
  NOT_FOUND = "Not found",
  UNHANDLED = "Unhandled",
}

export type ResponseError =
  | {type: ResponseErrorType.NOT_FOUND; message?: string}
  | {type: ResponseErrorType.UNHANDLED; message: string};

export async function withResponseError<T>(promise: Promise<T>): Promise<T> {
  return await promise.catch((error) => {
    console.error("ERROR!", error, typeof error);
    if (typeof error == "object" && "status" in error) {
      // This is a request!
      error = error as Response;
      if (error.status === 404) {
        throw {type: ResponseErrorType.NOT_FOUND};
      }
    }

    throw {
      type: ResponseErrorType.UNHANDLED,
      message: error.toString(),
    };
  });
}
