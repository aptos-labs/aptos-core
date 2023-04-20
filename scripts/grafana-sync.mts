#!/usr/bin/env -S node --loader ts-node/esm

import { assert } from "console";
import "zx/globals";

// This script syncs dashboards from grafana to a local directory and vice-versa.
// It is intended to be run from the root of the repo.
// TODO: Implement auto-deletion of dashboards in grafana which are not present in the local directory anymore.
// Usage:
// Prereqesuite: `export GRAFANA_API_KEY="<api_key>"` - Obtain it from <GRAFANA_URL>/grafana/org/apikeys in the Grafana UI
// Option 1. Download dashboards from grafana to local directory: ./scripts/grafana-sync.mts download
// Option 2. Upload dashboards from local directory to grafana: ./scripts/grafana-sync.mts upload

$.verbose = false;

const GRAFANA_URL = process.env.GRAFANA_URL ?? "https://aptoslabs.grafana.net";
const GRAFANA_DASHBOARD_FOLDER_NAME_TO_SYNC = process.env.GRAFANA_DASHBOARD_FOLDER_NAME_TO_SYNC ?? "aptos-core";
const LOCAL_DASHBOARD_FOLDER_TO_SYNC = process.env.LOCAL_DASHBOARD_FOLDER_TO_SYNC ?? "dashboards";

enum Action {
  Download = "download",
  Upload = "upload",
}

const positionalArgs = argv._;

const grafanaApiKey = process.env.GRAFANA_API_KEY;
assert(grafanaApiKey, "GRAFANA_API_KEY env var must be set");

const commonHeaders = {
  accept: "application/json",
  "content-type": "application/json",
  authorization: `Bearer ${grafanaApiKey}`,
};

switch (positionalArgs[0] as Action | unknown) {
  case Action.Download:
    await downloadAndWriteDashboardFolder(GRAFANA_DASHBOARD_FOLDER_NAME_TO_SYNC, LOCAL_DASHBOARD_FOLDER_TO_SYNC);
    break;
  case Action.Upload:
    await uploadDashboardsToGrafana(LOCAL_DASHBOARD_FOLDER_TO_SYNC);
    break;
  default:
    throw new Error("Invalid script action - must be one of: " + Object.values(Action).join(", "));
}

// REUSE FUNCTIONS
async function getGrafanaFolderByName(folderName: string): Promise<any> {
  const allFolders = await fetch(`${GRAFANA_URL}/api/folders`, {
    headers: commonHeaders,
  }).then(async (response) => {
    if (!response.ok) {
      throw new Error(`Failed to fetch grafana folders: ${response.statusText}: ${await response.text()}`);
    }
    return response.json();
  });

  const folderOfInterest = allFolders.find((folder) => folder.title === folderName);

  if (!folderOfInterest) {
    throw new Error(`Could not find folder with name ${folderName}`);
  }

  return folderOfInterest;
}

async function downloadDashboard(dashboardUid) {
  const response = await fetch(`${GRAFANA_URL}/api/dashboards/uid/${dashboardUid}`, {
    headers: commonHeaders,
  });

  const responseJson = await response.json();

  return responseJson.dashboard;
}

async function downloadAndWriteDashboardFolder(grafanaFolderName: string, targetDir = "dashboards") {
  const grafanaFolder = await getGrafanaFolderByName(grafanaFolderName);

  const allFolderItems = await fetch(`${GRAFANA_URL}/api/search?folderIds=${grafanaFolder.id}`, {
    headers: commonHeaders,
  }).then((response) => response.json());

  const dashboardFolderItems = allFolderItems.filter((item) => item.type === "dash-db");

  const dashboardFilesToUpdate = new Set(await glob([`${targetDir}/*.json`]));

  await Promise.all(
    dashboardFolderItems.map(async (dashboardFolderItem) => {
      const targetFilePath = `${targetDir}/${
        dashboardFolderItem.title
          .toLowerCase()
          .replaceAll(" ", "-")
          .replaceAll("(", "")
          .replaceAll(")", "")
          .replaceAll(".", "-") + ".json"
      }`;

      console.log(
        `Downloading dashboard: ${dashboardFolderItem.title} (${dashboardFolderItem.uid}) - ${targetFilePath}`,
      );

      const dashboard = await downloadDashboard(dashboardFolderItem.uid);

      // remove `id` property since its value is grafana installation specific and shouldn't be in the sources therefore. See https://grafana.com/docs/grafana/v9.0/developers/http_api/dashboard/#identifier-id-vs-unique-identifier-uid
      delete dashboard.id;

      await fs.writeJson(targetFilePath, dashboard);
      await $`prettier --write ${targetFilePath}`;
      dashboardFilesToUpdate.delete(targetFilePath);
    }),
  );

  if (dashboardFilesToUpdate.size > 0) {
    console.warn("Found dashboards in the repo which are not present in Grafana - deleting them now from the repo:");
    for (const dashboardFilePath of dashboardFilesToUpdate) {
      console.warn(dashboardFilePath);
      await fs.remove(dashboardFilePath);
      await fs.remove(dashboardFilePath + ".gz");
    }
  }

  // HACK: Compress dashboards for better consumption in helm to avoid reaching configmap 1MB limit of our grafana/monitoring chart
  await $`gzip -fkn ${LOCAL_DASHBOARD_FOLDER_TO_SYNC}/*.json`;
  console.info("successfully downloaded dashboards from grafana folder:", grafanaFolderName);
}

async function readLocalDashboards(localDashboardFolder: string) {
  let dashboardFilePaths = await glob([`${localDashboardFolder}/*.json`]);
  return await Promise.all(dashboardFilePaths.map(async (filePath) => [filePath, await fs.readJson(filePath)]));
}

async function uploadDashboardsToGrafana(localDashboardFolder: string) {
  const grafanaFolder = await getGrafanaFolderByName(GRAFANA_DASHBOARD_FOLDER_NAME_TO_SYNC);

  for (const [filePath, dashboard] of await readLocalDashboards(localDashboardFolder)) {
    console.info("uploading dashboard:", filePath);
    await uploadDashboard(dashboard, grafanaFolder.uid);
  }
  console.info("successfully uploaded dashboards to grafana");
}

async function uploadDashboard(dashboard: any, grafanaFolderUid: string) {
  dashboard.id = null;

  dashboard.title = dashboard.title;
  const response = await fetch(`${GRAFANA_URL}/api/dashboards/db`, {
    method: "POST",
    headers: commonHeaders,
    body: JSON.stringify({
      dashboard,
      folderId: 0,
      folderUid: grafanaFolderUid,
      message: "changes made by grafana-sync.mts",
      overwrite: true,
    }),
  });

  if (!response.ok) {
    console.error(await response.text());
    process.exit(1);
  }
}
