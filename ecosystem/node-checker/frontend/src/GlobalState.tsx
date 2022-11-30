import React from "react";
import {
  networks,
  defaultNetworkName,
  NetworkName,
  features,
  defaultFeatureName,
  FeatureName,
} from "./constants";

const selected_network = safeGetSelectedNetworkName();

function safeGetSelectedNetworkName(): NetworkName {
  let selected_network = localStorage.getItem("selected_network");
  if (selected_network) {
    selected_network = selected_network.toLowerCase();
    if (selected_network in networks) {
      return selected_network as NetworkName;
    }
  }
  return defaultNetworkName;
}

const selected_feature = safeGetSelectedFeatureName();

function safeGetSelectedFeatureName(): FeatureName {
  let selected_feature = localStorage.getItem("selected_feature");
  if (selected_feature) {
    selected_feature = selected_feature.toLowerCase();
    if (selected_feature in features) {
      return selected_feature as FeatureName;
    }
  }
  return defaultFeatureName;
}

export type GlobalState = {
  network_name: NetworkName;
  network_value: string;
  feature_name: FeatureName;
};

const defaultGlobalState: GlobalState = {
  network_name: selected_network,
  network_value: networks[selected_network],
  feature_name: selected_feature,
};

function reducer(state: GlobalState, newValue: GlobalState): GlobalState {
  if (newValue.network_name) {
    localStorage.setItem(
      "selected_network",
      newValue.network_name.toLowerCase(),
    );
  }
  if (newValue.feature_name) {
    localStorage.setItem(
      "selected_feature",
      newValue.feature_name.toLowerCase(),
    );
  }
  return {...state, ...newValue};
}

export const GlobalStateContext = React.createContext(defaultGlobalState);
export const DispatchStateContext = React.createContext<
  React.Dispatch<GlobalState>
>((value: GlobalState) => value);

export const GlobalStateProvider = ({
  children,
}: {
  children: React.ReactNode;
}) => {
  const [state, dispatch] = React.useReducer(reducer, defaultGlobalState);
  return (
    <GlobalStateContext.Provider value={state}>
      <DispatchStateContext.Provider value={dispatch}>
        {children}
      </DispatchStateContext.Provider>
    </GlobalStateContext.Provider>
  );
};

export const useGlobalState = (): [
  GlobalState,
  React.Dispatch<GlobalState>,
] => [
  React.useContext(GlobalStateContext),
  React.useContext(DispatchStateContext),
];
