import React from "react";
import "./App.css";

import { Types, AptosClient } from "aptos";

const client = new AptosClient("https://fullnode.devnet.aptoslabs.com");

/** Convert string to hex-encoded utf-8 bytes. */
function stringToHex(text: string) {
  const encoder = new TextEncoder();
  const encoded = encoder.encode(text);
  return Array.from(encoded, (i) => i.toString(16).padStart(2, "0")).join("");
}

function App() {
  // Retrieve aptos.account on initial render and store it.
  const urlAddress = window.location.pathname.slice(1).toLowerCase();
  const isEditable = !urlAddress;
  const [address, setAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (urlAddress) {
      setAddress(urlAddress);
    } else {
      window.aptos.account().then(setAddress);
    }
  }, [urlAddress]);

  // Check for the module; show publish instructions if not present.
  const [modules, setModules] = React.useState<Types.MoveModule[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountModules(address).then(setModules);
  }, [address]);

  const hasModule = modules.some((m) => m.abi?.name === "Message");
  const publishInstructions = (
    <pre>
      Run this command to publish the module:
      <br />
      aptos move publish --package-dir /path/to/hello_blockchain/
      --named-addresses HelloBlockchain={address}
    </pre>
  );

  // Call set_message with the textarea value on submit.
  const ref = React.createRef<HTMLTextAreaElement>();
  const [isSaving, setIsSaving] = React.useState(false);
  const handleSubmit = async (e: any) => {
    e.preventDefault();
    if (!ref.current) return;

    const message = ref.current.value;
    const transaction = {
      type: "script_function_payload",
      function: `${address}::Message::set_message`,
      arguments: [stringToHex(message)],
      type_arguments: [],
    };

    try {
      setIsSaving(true);
      await window.aptos.signAndSubmitTransaction(transaction);
    } finally {
      setIsSaving(false);
    }
  };

  // Get the message from account resources.
  const [resources, setResources] = React.useState<Types.AccountResource[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountResources(address).then(setResources);
  }, [address]);
  const resourceType = `${address}::Message::MessageHolder`;
  const resource = resources.find((r) => r.type === resourceType);
  const data = resource?.data as { message: string } | undefined;
  const message = data?.message;

  return (
    <div className="App">
      {hasModule ? (
        <form onSubmit={handleSubmit}>
          <textarea ref={ref} defaultValue={message} readOnly={!isEditable} />
          {isEditable && <input disabled={isSaving} type="submit" />}
          {isEditable && <a href={address!}>Get public URL</a>}
        </form>
      ) : (
        publishInstructions
      )}
    </div>
  );
}

export default App;
