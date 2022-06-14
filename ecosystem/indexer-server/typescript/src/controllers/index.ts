import { ActivityControllerController } from './ActivityControllerController';
import { DefaultController } from './DefaultController';
import { GeneralController } from './GeneralController';

export const controllers = {
  ActivityControllerController,
  DefaultController,
  GeneralController,
};

type RecursiveKeyof<T> = T extends object ? (
  T extends readonly any[] ? RecursiveKeyof<T[number]> : (
    keyof T | RecursiveKeyof<T[keyof T]>
  )
) : never;

export type ControllerType = keyof typeof controllers;

export type ControllerServiceType = Exclude<RecursiveKeyof<typeof controllers>, ControllerType>;

export default {
  ActivityControllerController,
  DefaultController,
  GeneralController,
};
