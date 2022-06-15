/* eslint-disable import/no-named-as-default */
/* eslint-disable no-async-promise-executor */
/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable camelcase */
/* eslint-disable no-unused-vars */
import Service from './Service';
/**
* Ledger information
*
* returns LedgerInfo
* */
const get_ledger_info = () => new Promise(
  async (resolve, reject) => {
    try {
      resolve(Service.successResponse({
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);
/**
* API document
*
* no response value expected for this operation
* */
const get_spec_html = () => new Promise(
  async (resolve, reject) => {
    try {
      resolve(Service.successResponse({
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);
/**
* OpenAPI specification
*
* no response value expected for this operation
* */
const get_spec_yaml = () => new Promise(
  async (resolve, reject) => {
    try {
      resolve(Service.successResponse({
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

export default {
  get_ledger_info,
  get_spec_html,
  get_spec_yaml,
};
