require("dotenv").config();

const cli = require("@aptos-labs/ts-sdk/dist/common/cli/index.js");

async function test() {
  const move = new cli.Move();

  await move.test({
    packageDirectoryPath: "contract",
    namedAddresses: {
      todolist_addr: process.env.VITE_MODULE_PUBLISHER_ACCOUNT_ADDRESS,
    },
  });
}
test();
