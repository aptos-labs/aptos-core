/* eslint-disable @typescript-eslint/naming-convention */
/**
 * The GeneralController file is a very simple one, which does not need to be changed manually,
 * unless there's a case where business logic routes the request to an entity which is not
 * the service.
 * The heavy lifting of the Controller item is done in Request.js - that is where request
 * parameters are extracted and sent to the service, and where response is handled.
 */

import Controller from './Controller';
import service from '../services/GeneralService';

export const get_ledger_info = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.get_ledger_info);
};

export const get_spec_html = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.get_spec_html);
};

export const get_spec_yaml = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.get_spec_yaml);
};

export const GeneralController = {
  get_ledger_info,
  get_spec_html,
  get_spec_yaml,
};

export type GeneralControllerType = keyof typeof GeneralController;

export default {
  get_ledger_info,
  get_spec_html,
  get_spec_yaml,
};
