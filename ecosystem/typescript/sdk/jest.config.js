/** @type {import("ts-jest/dist/types").InitialOptionsTsJest} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  coveragePathIgnorePatterns: ["generated/*", "transaction_builder/aptos_types/*"],
  testPathIgnorePatterns: ["dist/*"],
  collectCoverage: true,
  coverageThreshold: {
    global: {
      branches: 50, // 90,
      functions: 50, // 95,
      lines: 50, // 95,
      statements: 50, // 95,
    },
  },
};
