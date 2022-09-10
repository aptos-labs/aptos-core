#!/usr/bin/env node

require("dotenv").config();
const { build } = require("esbuild");

if (!process.env.NODE_ENV) {
  process.env.NODE_ENV = process.env.RAILS_ENV;
}

const define = ["NODE_ENV", "SENTRY_FRONTEND_DSN"].reduce((acc, env) => {
  acc[`process.env.${env}`] = JSON.stringify(process.env[env]);
  return acc;
}, {});

const options = {
  entryPoints: ["./app/javascript/application.ts"],
  bundle: true,
  minify: process.env.NODE_ENV === "production",
  target: "es2020",
  sourcemap: true,
  outdir: "./app/assets/builds/",
  define: define,
};

build(options).catch(() => process.exit(1));
