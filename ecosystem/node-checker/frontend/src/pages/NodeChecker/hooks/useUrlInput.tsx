import React, {useState, useEffect} from "react";
import UrlTextField from "../../../components/UrlTextField";

function isValidUrl(url: string): boolean {
  let parsedUrl;
  try {
    parsedUrl = new URL(url);
  } catch (_) {
    return false;
  }
  return parsedUrl.protocol === "http:" || parsedUrl.protocol === "https:";
}

const useUrlInput = () => {
  const [url, setUrl] = useState<string>("");
  const [urlIsValid, setUrlIsValid] = useState<boolean>(true);

  useEffect(() => {
    setUrlIsValid(true);
  }, [url]);

  const onUrlChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setUrl(event.target.value);
  };

  function clearUrl() {
    setUrl("");
  }

  function renderUrlTextField(label: string): JSX.Element {
    return (
      <UrlTextField
        label={label}
        url={url}
        urlIsValid={urlIsValid}
        errorMessage={`URL is invalid, ensure it starts with http:// or https://`}
        onUrlChange={onUrlChange}
      />
    );
  }

  function validateUrlInput(): boolean {
    const isValid = isValidUrl(url);
    setUrlIsValid(isValid);
    return isValid;
  }

  return {url, clearUrl, renderUrlTextField, validateUrlInput};
};

export default useUrlInput;
