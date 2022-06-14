/* eslint-disable no-console */
import config from './config';
import ExpressServer from './expressServer';

const launchServer = async () => {
  try {
    const expressServer = new ExpressServer(config.URL_PORT, config.OPENAPI_YAML);
    console.log(`
    🚀 Server ready at: http://localhost:${config.URL_PORT}
    ⭐️ See sample requests: http://localhost:${config.URL_PORT}/api-docs`);
    expressServer.launch();
    // logger.info('Express server running');
  } catch (error: any) {
    console.error(error);
    // logger.error('Express Server failure', error.message);
    await (this as any).close();
  }
};

launchServer().catch(
  (e) => {
    console.log(e);
    // logger.error(e)
  },
);
