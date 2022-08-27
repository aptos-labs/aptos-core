/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const HtmlWebpackPlugin = require('html-webpack-plugin');

// This will be populated with craPaths when `paths` callback is called
const craPaths = {};

// region Utils

const minifyConfig = {
  minify: {
    collapseWhitespace: true,
    keepClosingSlash: true,
    minifyCSS: true,
    minifyJS: true,
    minifyURLs: true,
    removeComments: true,
    removeEmptyAttributes: true,
    removeRedundantAttributes: true,
    removeStyleLinkTypeAttributes: true,
    useShortDoctype: true,
  },
};

// endregion

module.exports = {
  paths(defaultPaths) {
    // Intercept CRA paths so that we can use them during webpack configuration
    Object.assign(craPaths, defaultPaths);
    return defaultPaths;
  },

  /**
   * The Webpack config to use when compiling your react app for development or production.
   */
  webpack(config, env) {
    const isEnvProduction = env === 'production';

    // region Replace default entrypoint with custom set of entrypoints

    // Original for reference:
    // // These are the "entry points" to our application.
    // // This means they will be the "root" imports that are included in JS bundle.
    // entry: paths.appIndexJs,

    Object.assign(config, {
      entry: {
        background: `${craPaths.appSrc}/scripts/background.ts`,
        contentscript: `${craPaths.appSrc}/scripts/contentscript.ts`,
        core: {
          import: `${craPaths.appSrc}/core`,
        },
        inpage: `${craPaths.appSrc}/scripts/inpage.ts`,
        main: {
          dependOn: ['core'],
          import: `${craPaths.appSrc}/index.tsx`,
        },
        prompt: {
          dependOn: ['core'],
          import: `${craPaths.appSrc}/scripts/prompt.tsx`,
        },
      },
    });

    // endregion

    // region Name of bundles should be unique for each entrypoint in development as well

    // Original for reference:
    // // There will be one main bundle, and one file per asynchronous chunk.
    // // In development, it does not produce real files.
    // filename: isEnvProduction
    //   ? 'static/js/[name].[contenthash:8].js'
    //   : isEnvDevelopment && 'static/js/bundle.js',

    Object.assign(config.output, {
      filename: 'static/js/[name].js',
    });

    // endregion

    Object.assign(config.resolve, {
      fallback: {
        stream: require.resolve('stream-browserify'),
      },
    });

    // region Replace default HtmlWebpackPlugin entry with updated ones

    const htmlPluginIdx = config.plugins.findIndex((plugin) => plugin instanceof HtmlWebpackPlugin);
    const defaultHtmlPluginFound = htmlPluginIdx >= 0;
    if (!defaultHtmlPluginFound) {
      // eslint-disable-next-line no-console
      console.log('Warning: Default HtmlWebpackPlugin not found!');
    }

    function getChunksFromEntry(entry) {
      const chunks = [entry];
      const entrypoint = config.entry[entry];
      if (entrypoint.dependOn) {
        entrypoint.dependOn.forEach((dep) => {
          chunks.push(...getChunksFromEntry(dep));
        });
      }
      return chunks;
    }

    function makeHtmlPlugin(entry, filename) {
      const envSpecificConfig = isEnvProduction ? {
        ...minifyConfig,
      } : undefined;

      const chunks = getChunksFromEntry(entry);
      return new HtmlWebpackPlugin(
        {
          chunks,
          filename,
          inject: true,
          template: `${craPaths.appPublic}/${filename}`,
          ...envSpecificConfig,
        },
      );
    }

    const targetIdx = defaultHtmlPluginFound ? htmlPluginIdx : 0;
    config.plugins.splice(
      targetIdx,
      defaultHtmlPluginFound ? 1 : 0,
      makeHtmlPlugin('main', 'index.html'),
      makeHtmlPlugin('prompt', 'prompt.html'),
    );

    // endregion

    // region Add fallback plugins

    // Needed for the aptos SDK
    Object.assign(config.resolve, {
      fallback: {
        events: false,
        stream: require.resolve('stream-browserify'),
      },
    });

    if (isEnvProduction) {
      // The `Buffer` class is available as built-in global when running the webpack dev server,
      // but is not imported automatically in builds. This makes sure that
      // `import { Buffer } from 'buffer'` is added to every entry point.
      config.plugins.push(new webpack.ProvidePlugin({
        Buffer: ['buffer', 'Buffer'],
      }));
    }

    // endregion

    return config;
  },
};
