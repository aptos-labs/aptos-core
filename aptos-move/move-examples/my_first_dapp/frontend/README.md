## Create Aptos Dapp Boilerplate Template

The Boilerplate template provides a starter dapp with all necessary dapp infrastructure and a simple wallet info implementation, transfer APT and a simple message board functionality to send and read a message on chain.

## Read the Boilerplate template docs

To get started with the Boilerplate template and learn more about the template functionality and usage, head over to the [Boilerplate template docs](https://learn.aptoslabs.com/en/dapp-templates/boilerplate-template)

## The Boilerplate template provides:

- **Folder structure** - A pre-made dapp folder structure with a `frontend` and `contract` folders.
- **Dapp infrastructure** - All required dependencies a dapp needs to start building on the Aptos network.
- **Wallet Info implementation** - Pre-made `WalletInfo` components to demonstrate how one can use to read a connected Wallet info.
- **Transfer APT implementation** - Pre-made `transfer` components to send APT to an address.
- **Message board functionality implementation** - Pre-made `message` components to send and read a message on chain

## What tools the template uses?

- React framework
- Vite development tool
- shadcn/ui + tailwind for styling
- Aptos TS SDK
- Aptos Wallet Adapter
- Node based Move commands
- [Vite-pwa](https://vite-pwa-org.netlify.app/)

## What Move commands are available?

The tool utilizes [aptos-cli npm package](https://github.com/aptos-labs/aptos-cli) that lets us run Aptos CLI in a Node environment.

Some commands are built-in the template and can be ran as a npm script, for example:

- `npm run move:publish` - a command to publish the Move contract
- `npm run move:test` - a command to run Move unit tests
- `npm run move:compile` - a command to compile the Move contract
- `npm run move:upgrade` - a command to upgrade the Move contract
- `npm run dev` - a command to run the frontend locally
- `npm run deploy` - a command to deploy the dapp to Vercel

For all other available CLI commands, can run `npx aptos` and see a list of all available commands.
