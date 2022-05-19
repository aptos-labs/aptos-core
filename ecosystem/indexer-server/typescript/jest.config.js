/** @type {import("ts-jest/dist/types").InitialOptionsTsJest} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  coveragePathIgnorePatterns: ["api/*"],
  testPathIgnorePatterns: ["dist/*"],
  collectCoverage: true,
  coverageThreshold: {
    global: {
      branches: 80, // 90,
      functions: 60, // 95,
      lines: 60, // 95,
      statements: 60, // 95,
    },
  },
};
