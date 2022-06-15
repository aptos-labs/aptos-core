import { Router } from 'express';
import { controllers, ControllerServiceType, ControllerType } from '../controllers';

interface CreateRouterParams {
  schema: any;
}

const createRouter = ({
  schema,
}: CreateRouterParams) => {
  const router = Router();
  Object.entries(schema.paths).forEach(([openApiPath, pathObject]: [string, any]) => {
    // may need matchAll in the future for paths with multiple ids
    const substring = openApiPath.match(/{.*}/)?.toString();
    // custom regex to exclude the <resource>/all path
    const routerPath = (substring)
      ? openApiPath.replace(/{.*}/, `:${substring}`).replace('{', '').replace('}', '')
      : openApiPath;

    if (pathObject.post) {
      const controllerName: ControllerType | undefined = pathObject?.post['x-eov-operation-handler']?.replace('controllers/', '');
      const serviceName: ControllerServiceType | undefined = pathObject.post.operationId;
      if (controllerName && serviceName) {
        router.post(routerPath, (controllers as any)[controllerName][serviceName]);
      }
    } else if (pathObject.get) {
      const controllerName: ControllerType | undefined = pathObject?.get['x-eov-operation-handler']?.replace('controllers/', '');
      const serviceName: ControllerServiceType | undefined = pathObject.get.operationId;
      if (controllerName && serviceName) {
        router.get(routerPath, (controllers as any)[controllerName][serviceName]);
      }
    }
  });
  return router;
};

export default createRouter;
