import express from 'express';
import services from '../services';
import { jsonStringify } from '../utils/jsonStringify';

class Controller {
  static sendResponse(response: {
    end: (arg0: any) => void;
    json: (arg0: any) => void;
    status: (arg0: any) => void;
  }, payload: {
    code: any; payload: undefined;
  }) {
    /**
    * The default response-code is 200. We want to allow to change that. in That case,
    * payload will be an object consisting of a code and a payload. If not customized
    * send 200 and the payload as received in this method.
    */
    response.status(payload.code || 200);
    const responsePayload = payload.payload !== undefined ? payload.payload : payload;
    if (responsePayload instanceof Object) {
      response.json(JSON.parse(JSON.stringify(responsePayload, jsonStringify)));
    } else {
      response.end(responsePayload);
    }
  }

  static sendError(response: {
    end: (arg0: any) => void;
    json: (arg0: any) => void;
    status: (arg0: any) => void;
  }, error: any) {
    response.status(error.code || 500);
    if (error.error instanceof Object) {
      response.json(JSON.parse(JSON.stringify(error.error, jsonStringify)));
    } else {
      response.end(error.error || error.message);
    }
  }

  static getRequestBodyName(
    request: any,
  ) {
    const codeGenDefinedBodyName = request.openapi.schema['x-codegen-request-body-name'];
    if (codeGenDefinedBodyName !== undefined) {
      return codeGenDefinedBodyName;
    }
    const refObjectPath = request.openapi.schema.requestBody.content['application/json'].schema.$ref;
    if (refObjectPath !== undefined && refObjectPath.length > 0) {
      return (refObjectPath.substr(refObjectPath.lastIndexOf('/') + 1));
    }
    return 'body';
  }

  static collectRequestParams(
    request: express.Request,
  ) {
    // TODO double check we dont need request.headers or request.body
    const requestParams: Record<string, any> = {
      ...request.query,
      ...request.params,
      ...request.body,
    };
    return requestParams;
  }

  static async handleRequest(
    request: express.Request,
    response: any,
    serviceOperation: (arg0: any) => any,
  ) {
    try {
      const requestParams = this.collectRequestParams(request);
      let serviceResponse;
      if (request?.route?.path === '/collections/:collectionId' && request.url === '/collections/all') {
        serviceResponse = await services.DefaultService.getAllCollections(requestParams);
      } else {
        serviceResponse = await serviceOperation(requestParams);
      }
      Controller.sendResponse(response, serviceResponse);
    } catch (error) {
      Controller.sendError(response, error);
    }
  }
}

export default Controller;
