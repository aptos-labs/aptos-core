export interface SuccessResponseType<T> {
  code: number;
  payload: T;
}

class Service {
  static rejectResponse(error: any, code = 500) {
    return { code, error };
  }

  static successResponse<T>(payload: any, code = 200): SuccessResponseType<T> {
    return { code, payload };
  }
}

export default Service;
