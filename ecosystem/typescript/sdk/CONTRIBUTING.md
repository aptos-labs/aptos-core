# Contribution Guidelines for Typescript SDK

- Coding Styles
  - File names must use Snake case. For example, `aptos_account.ts` .
  - Class names must use Pascal case. For example, `class AuthenticationKey` .
  - Function and method names must use Camel case. For example, `derivedAddress(): HexString` .
  - Constants must use all caps (upper case) words separated by `_`. For example, `MAX_U8_NUMBER` .
- Comments
  - Comments are required for new classes and functions.
  - Comments should follow the TSDoc standard, [https://tsdoc.org/](https://tsdoc.org/).
- Lints and Formats
  - ESlint (eslint) and Prettier (prettier) should be used for code checking and code formatting. Make sure to run `yarn lint` and `yarn fmt` after making changes to the code.
- Tests
  - Unit tests are required for any non-trivial changes you make.
  - The Jest testing framework is used in the repo and we recommend you use it. See Jest: [https://jestjs.io/](https://jestjs.io/).
  - Make sure to run `yarn test` after making changes.
- Commits
  - Commit messages follow the [Angular convention](https://www.conventionalcommits.org/en/v1.0.0-beta.4/#summary).
