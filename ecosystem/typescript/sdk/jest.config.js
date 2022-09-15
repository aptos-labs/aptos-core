/** @type {import("ts-jest/dist/types").InitialOptionsTsJest} */
module.exports = {
  preset: "ts-jest",
  moduleNameMapper: {
    "^(\\.{1,2}/.*)\\.js$": "$1",
  },
  testEnvironment: "node",
  coveragePathIgnorePatterns: ["generated/*", "./aptos_types/*", "utils/memoize-decorator.ts", "utils/hd-key.ts"],
  testPathIgnorePatterns: ["dist/*"],
  collectCoverage: true,
  setupFiles: ["dotenv/config"],
  coverageThreshold: {
    global: {
      branches: 50, // 90,
      functions: 50, // 95,
      lines: 50, // 95,
      statements: 50, // 95,
    },
  },
};
