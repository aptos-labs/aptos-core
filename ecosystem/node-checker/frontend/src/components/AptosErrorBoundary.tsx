import React from "react";
import {ErrorBoundary} from "@sentry/react";

export function AptosErrorBoundary<T>(props: React.PropsWithChildren<T>) {
  return <ErrorBoundary {...props} />;
}
