---
title: "Integrate with Aptos NameService UI Package"
id: "ans-ui-package"
---
# Aptos Name Service UI Package
The Aptos Name Service provides a React UI package that provides developers with a customizable button and modal to enable users to search for and mint Aptos names directly from their website.

## Usage

To use the Aptos Name Service Connector component, you will need to install it using npm or yarn:
```
npm install "@aptos-labs/aptos-name-connector"
```


Once you have installed the package, you can import the `AnsConnector` component and use it in your React application:

```
import { AnsConnector } from "@aptos-labs/aptos-name-connector";

function MyComponent() {
  const handleSignTransaction = async () => {
    // Handle signing of transaction
  };

  return (
    <AnsConnector
      onSignTransaction={handleSignTransaction}
      isWalletConnected={true}
      network="mainnet"
    />
  );
}
```

## Props
The `AnsConnector` component accepts the following props:

- onSignTransaction: A required callback function that is called when the user clicks the "Mint" button in the modal. This function should handle the signing of the transaction.
- isWalletConnected: A boolean value that indicates whether the user's wallet is connected.
- network: A string value that specifies whether the component should connect to the mainnet or testnet.
- buttonLabel: A string value that specifies the text to display on the button.

## Customization
The button label can be customized by passing a string value to the buttonLabel prop.
The appearance of the button in the `AnsConnector` component can be customized to fit in your website. The button has the following class names:

ans_connector_button: The css class name for the button.

```
.ans-connector-button {
  background-color: #000000;
  border: none;
  border-radius: 4px;
  color: #ffffff;
  cursor: pointer;
  font-size: 16px;
  font-weight: bold;
  padding: 12px 16px;
}
```

## Supported Networks
The `AnsConnector` component supports both mainnet and testnet. To connect to the mainnet, set the network prop to "mainnet". To connect to the testnet, set the network prop to "testnet".

## Example
The following example shows how to use the AnsConnector component in a React application:
<last image>


- Add a ‘claim name’ button to any page in your application. This allows your users to directly create an Aptos name, giving them a human-readable .apt name for their Aptos wallet address. You can customize the look of the button to suit your application. Here is an example on the profile page of an NFT marketplace.

![Claim name](../../static/img/docs/ans_entrypoint_example.png)

- When the button is clicked, the Aptos Names modal will show up, and the user can search for a name and mint it directly in your application.

![Show Aptos Name Service modal](../../static/img/docs/ans_entrypoint_modal_example.png)

- Once the user has minted their name, you can replace their Aptos wallet address by querying from Aptos fullnodes. Now your users have a human-readable .apt name.

![Claim another name](../../static/img/docs/ans_entrypoint_with_other_name.png)
