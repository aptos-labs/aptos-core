# Diem Dashboard Templates

The dashboard templates are loaded as ConfigMap in compressed binary format because of the ConfigMap size limit.

To update the dashboard template, you should update the content of the `.json` file and then gzip them into the binary file: `gzip -fkn *.json`
