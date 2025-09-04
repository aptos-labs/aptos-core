# Velor Dashboard Templates

The dashboard templates are loaded as ConfigMap in compressed binary format because of the ConfigMap size limit.

To update the dashboard template, you should update the content of the `.json` file and then gzip them into the binary file: `gzip -fkn *.json`

Symlink this directory to anywhere you need dashboards.

## Updating dashboards

This directory is automatically synced with the Velor Labs central Grafana service. A Github Action will periodically scan that Grafana and create a PR to update the dashboards in this repository.
