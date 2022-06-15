/**
 * The DefaultController file is a very simple one, which does not need to be changed manually,
 * unless there's a case where business logic routes the request to an entity which is not
 * the service.
 * The heavy lifting of the Controller item is done in Request.js - that is where request
 * parameters are extracted and sent to the service, and where response is handled.
 */

import Controller from './Controller';
import service from '../services/DefaultService';

export const getAllCollections = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getAllCollections);
};

export const getAllOwnerships = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getAllOwnerships);
};

export const getAllTokens = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getAllTokens);
};

export const getCollectionById = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getCollectionById);
};

export const getOwnershipById = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getOwnershipById);
};

export const getOwnershipsByIds = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getOwnershipsByIds);
};

export const getOwnershipsByToken = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getOwnershipsByToken);
};

export const getTokenById = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getTokenById);
};

export const getTokenByIds = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getTokenByIds);
};

export const getTokenMetaDataById = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getTokenMetaDataById);
};

export const getTokenMetaDataByIds = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getTokenMetaDataByIds);
};

export const getTokenRoyaltiesById = async (request: any, response: any) => {
  await Controller.handleRequest(request, response, service.getTokenRoyaltiesById);
};

export const DefaultController = {
  getAllCollections,
  getAllOwnerships,
  getAllTokens,
  getCollectionById,
  getOwnershipById,
  getOwnershipsByIds,
  getOwnershipsByToken,
  getTokenById,
  getTokenByIds,
  getTokenMetaDataById,
  getTokenMetaDataByIds,
  getTokenRoyaltiesById,
};

export type DefaultControllerType = keyof typeof DefaultController;

export default {
  getAllCollections,
  getAllOwnerships,
  getAllTokens,
  getCollectionById,
  getOwnershipById,
  getOwnershipsByIds,
  getOwnershipsByToken,
  getTokenById,
  getTokenByIds,
  getTokenMetaDataById,
  getTokenMetaDataByIds,
  getTokenRoyaltiesById,
};
