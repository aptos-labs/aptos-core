import React, {useEffect, useState} from "react";
import {Button, Grid, Typography} from "@mui/material";
import HeadingSub from "../../components/HeadingSub";
import {ApiError, EvaluationSummary} from "aptos-node-checker-client";
import useUrlInput from "./hooks/useUrlInput";
import usePortInput from "./hooks/usePortInput";
import EvaluationDisplay from "./EvaluationDisplay";
import {checkNode, determineNhcUrl, MinimalConfiguration} from "./Client";
import ConfigurationSelect from "./ConfigurationSelect";
import {useGlobalState} from "../../GlobalState";
import ErrorSnackbar from "./ErrorSnackbar";
import useAddressInput from "../../api/hooks/useAddressInput";

export function NodeCheckerPage() {
  const [state, _dispatch] = useGlobalState();

  const [checking, updateChecking] = useState<boolean>(false);
  const [evaluationSummary, updateEvaluationSummary] = useState<
    EvaluationSummary | undefined
  >(undefined);
  const [baselineConfiguration, updateBaselineConfiguration] = useState<
    MinimalConfiguration | undefined
  >(undefined);
  const [errorMessage, updateErrorMessage] = useState<string | undefined>(
    undefined,
  );
  const [publicKeyRequired, setPublicKeyRequired] = useState<boolean>(true);

  // URL text input field.
  const {
    url,
    clearUrl: _clearUrl,
    renderUrlTextField,
    validateUrlInput,
  } = useUrlInput();

  // API port text input field.
  const {
    port: apiPort,
    clearPort: _clearApiPort,
    renderPortTextField: renderApiPortTextField,
    validatePortInput: validateApiPortInput,
  } = usePortInput("8080");

  // Noise port text input field.
  const {
    port: noisePort,
    clearPort: _clearNoisePort,
    renderPortTextField: renderNoisePortTextField,
    validatePortInput: validateNoisePortInput,
  } = usePortInput("6180");

  // Public key text input field.
  const {
    addr: publicKey,
    clearAddr: _clearPublicKey,
    renderAddressTextField: renderPublicKeyTextField,
    validateAddressInput: validatePublicKeyAddressInput,
  } = useAddressInput();

  const nhcUrl = determineNhcUrl(state);

  // Check whether all the input fields are valid.
  const inputIsValid = () => {
    const urlIsValid = validateUrlInput();
    const apiPortIsValid = validateApiPortInput();
    const noisePortIsValid = validateNoisePortInput();
    const publicKeyIsValid = publicKeyRequired
      ? validatePublicKeyAddressInput()
      : true;
    return (
      urlIsValid &&
      apiPortIsValid &&
      noisePortIsValid &&
      publicKeyIsValid &&
      baselineConfiguration !== undefined
    );
  };

  // Wrapper around updateBaselineConfiguration that also handles whether
  // the public key field is required.
  const updateBaselineConfigurationWrapper = (
    configuration: MinimalConfiguration | undefined,
  ) => {
    updateBaselineConfiguration(configuration);
    const evaluators = configuration?.evaluators ?? [];
    setPublicKeyRequired(evaluators.includes("noise_handshake"));
  };

  const onCheckNodeButtonClick = async () => {
    if (checking) {
      return;
    }
    if (!inputIsValid()) {
      return;
    }
    updateChecking(true);
    try {
      const evaluationSummary = await checkNode({
        nhcUrl: nhcUrl,
        nodeUrl: url,
        baselineConfigurationName: baselineConfiguration!.name,
        // TODO: Somehow make these port values numbers to begin with.
        apiPort: parseInt(apiPort),
        noisePort: parseInt(noisePort),
        publicKey: publicKey == "" ? undefined : publicKey,
      });
      updateEvaluationSummary(evaluationSummary);
      updateErrorMessage(undefined);
    } catch (e) {
      let msg = `Failed to check node: ${e}`;
      if (e instanceof ApiError) {
        msg += `: ${e.body}`;
      }
      updateErrorMessage(msg);
    }
    updateChecking(false);
  };

  // Clear the results if the user changes the network.
  useEffect(() => {
    updateEvaluationSummary(undefined);
    updateErrorMessage(undefined);
  }, [state.network_name]);

  // Conditionally build an input field for the public key if the selected
  // baseline configuration has an evaluator that requires it.
  let publicKeyInput = null;
  if (publicKeyRequired) {
    publicKeyInput = (
      <Grid item md={12}>
        {renderPublicKeyTextField("Public Key")}
      </Grid>
    );
  }

  // Build the check node button, which could be disabled if we're actively
  // waiting for a response from the server.
  const checkNodeButton = (
    <span>
      <Button
        fullWidth
        variant="primary"
        onClick={onCheckNodeButtonClick}
        disabled={checking}
      >
        {checking ? "Checking node, please wait..." : "Check Node"}
      </Button>
    </span>
  );

  // Build a display of the evaluation summary if one has been received.
  let evaluationDisplay = null;
  if (evaluationSummary !== undefined) {
    evaluationDisplay = (
      <EvaluationDisplay evaluationSummary={evaluationSummary!} />
    );
  }

  return (
    <Grid container spacing={3}>
      <ErrorSnackbar
        errorMessage={errorMessage}
        updateErrorMessage={updateErrorMessage}
      />
      <Grid item xs={12}>
        <HeadingSub>BETA</HeadingSub>
        <Typography variant="h1" component="h1" gutterBottom>
          Node Health Checker
        </Typography>
        <Grid container spacing={4}>
          <Grid item md={5} xs={12}>
            {renderUrlTextField("Node URL")}
          </Grid>
          <Grid item md={1.5} xs={12}>
            {renderApiPortTextField("API Port")}
          </Grid>
          <Grid item md={1.5} xs={12}>
            {renderNoisePortTextField("Noise Port")}
          </Grid>
          <Grid item md={4} xs={12}>
            <ConfigurationSelect
              baselineConfiguration={baselineConfiguration}
              updateBaselineConfiguration={updateBaselineConfigurationWrapper}
              updateErrorMessage={updateErrorMessage}
            />
          </Grid>
          {publicKeyInput}
          <Grid item xs={12}>
            {checkNodeButton}
          </Grid>
        </Grid>
        {evaluationDisplay}
      </Grid>
    </Grid>
  );
}
