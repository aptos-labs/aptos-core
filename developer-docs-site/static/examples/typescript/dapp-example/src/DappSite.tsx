import * as React from "react";
import { AptosClient, AptosAccount } from "aptos";

type Props = {
  userAddress: string | null;
  address: string;
};

const NODE_URL =
  process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const client = new AptosClient(NODE_URL);

function textToHex(text: string) {
  const encoder = new TextEncoder();
  const encoded = encoder.encode(text);
  return Array.from(encoded, (i) => i.toString(16).padStart(2, "0")).join("");
}

async function getContent(address: string) {
  const resources = await client.getAccountResources(address);
  const name = `${address}::Message::MessageHolder`;
  const messageHolder = resources.find((r) => r.type === name);
  const data: any = messageHolder?.data;
  if (data != null && "message" in data) {
    return data.message;
  } else {
    return `Create the module:
aptos move publish --package-dir /path/to/aptos-core/aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=${address}

...and then update this text`;
  }
}

async function submitContent(address: string, content: string) {
  const hexEncoded = textToHex(content);
  let payload: {
    function: string;
    arguments: any[];
    type: string;
    type_arguments: any[];
  };
  payload = {
    type: "script_function_payload",
    function: `${address}::Message::set_message`,
    type_arguments: [],
    arguments: [hexEncoded],
  };

  const txnRequest = await client.generateTransaction(address, payload);
  const result = await (window as any).aptos.signTransaction(txnRequest);
  const randomAcc = new AptosAccount();
  return client.submitTransaction(randomAcc, result);
}

export function DappSite({ userAddress, address }: Props) {
  const isEditable = userAddress != null && userAddress === address;

  const [content, setContent] = React.useState("");
  React.useEffect(() => {
    getContent(address).then((content) => {
      setContent(content);
    });
  }, [address]);

  const [saving, setSaving] = React.useState(false);
  const ref: any = React.createRef();
  const handleClick = () => {
    const content = ref.current.value;
    setSaving(true);
    submitContent(userAddress as string, content).then(() => {
      setSaving(false);
    });
  };

  return (
    <>
      {isEditable ? (
        <textarea
          ref={ref}
          defaultValue={content}
          style={{ border: 0, width: "100%", minHeight: "50vh", outline: 0 }}
        ></textarea>
      ) : (
        <pre>{content}</pre>
      )}
      {isEditable && (
        <button disabled={saving} onClick={handleClick}>
          Publish!
        </button>
      )}
      {isEditable && (
        <p>
          <code>
            <a href={userAddress}>Get public URL</a>
          </code>
        </p>
      )}
    </>
  );
}
