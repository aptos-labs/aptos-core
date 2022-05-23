import path from 'path';

interface ConfigType {
  BASE_VERSION: string;
  CONTROLLER_DIRECTORY: string;
  FILE_UPLOAD_PATH: string;
  FULL_PATH: string;
  OPENAPI_YAML: string;
  PROJECT_DIR: string;
  ROOT_DIR: string;
  URL_PATH: string;
  URL_PORT: number;
}

const config: ConfigType = {
  BASE_VERSION: '',
  CONTROLLER_DIRECTORY: path.join(__dirname, 'controllers'),
  FILE_UPLOAD_PATH: '',
  FULL_PATH: '',
  OPENAPI_YAML: '',
  PROJECT_DIR: __dirname,
  ROOT_DIR: __dirname,
  URL_PATH: 'http://localhost',
  URL_PORT: 4000,
};

config.OPENAPI_YAML = path.join(config.ROOT_DIR, 'api/v1', 'openapi.yaml');
config.FULL_PATH = `${config.URL_PATH}:${config.URL_PORT}/${config.BASE_VERSION}`;
config.FILE_UPLOAD_PATH = path.join(config.PROJECT_DIR, 'uploaded_files');

export default config;
