import json
import os
import requests

GRAFANA_URL = os.environ["GRAFANA_URL"]
API_KEY = os.environ["GRAFANA_API_KEY"]
if not GRAFANA_URL or not API_KEY:
    raise Exception("GRAFANA_URL and GRAFANA_API_KEY must be set")

GRAFANA_RESOURCE_PREFIX = "x-"  # so we don't collide with existing stuff

GRAFANA_REMOTE_FOLDER = f"{GRAFANA_RESOURCE_PREFIX}aptos-core"

PROMETHEUS_DATASOURCE = {
    "allFormat": "",
    "allValue": "",
    "current": {
        "selected": True,
        "text": "VictoriaMetrics Global (Non-mainnet)",
        "value": "VictoriaMetrics Global (Non-mainnet)",
    },
    "hide": 0,
    "includeAll": False,
    "label": "",
    "multi": False,
    "multiFormat": "",
    "name": "Datasource",
    "options": [],
    "query": "prometheus",
    "queryValue": "",
    "refresh": 1,
    "regex": ".*Prometheus.*|.*Victoria.*|.*Telemetry.*",
    "skipUrlSync": False,
    "sort": 0,
    "type": "datasource",
}

BIGQUERY_DATASOURCE = {
    "description": "BigQuery data source",
    "hide": 0,
    "includeAll": False,
    "multi": False,
    "name": "BigQuery",
    "options": [],
    "query": "grafana-bigquery-datasource",
    "refresh": 1,
    "regex": "",
    "skipUrlSync": False,
    "type": "datasource",
}

COMMON_HEADERS = {
    "accept": "application/json",
    "content-type": "application/json",
    "authorization": "Bearer " + API_KEY,
}


def get_remote_folder(folder_name):
    r = requests.get(
        f"{GRAFANA_URL}/api/folders",
        headers=COMMON_HEADERS,
    )
    r.raise_for_status()
    folders = r.json()
    for folder in folders:
        if folder["title"] == folder_name:
            return folder


def upload_dashboard(dashboard, folder):
    payload = {
        "dashboard": dashboard,
        "folderUid": folder["uid"],
        "overwrite": True,
        "message": "Migrated from old Grafana",
    }
    r = requests.post(
        f"{GRAFANA_URL}/api/dashboards/db",
        headers=COMMON_HEADERS,
        json=payload,
    )
    # r.raise_for_status()
    return r.json()


def replace_string(dashboard, old, new):
    """Cheaply replace a string in a dashboard JSON"""
    s = json.dumps(dashboard)
    s = s.replace(old, new)
    return json.loads(s)


def contains_string(dashboard, string):
    """Check if a dict contains a string"""
    s = json.dumps(dashboard)
    return string in s


def migrate_dashboard(dashboard_name):
    with open(f"{dashboard_name}.json") as f:
        dashboard = json.load(f)

    # prometheus datasource templates
    dashboard = replace_string(dashboard, "Zu4MxH4Vk", "${Datasource}")

    # bigquery datasource templates
    dashboard = replace_string(dashboard, "axNEitxVz", "${BigQuery}")
    dashboard = replace_string(dashboard, "P67060240DC0051B8", "${BigQuery}")

    # if the dashboard has a prometheus datasource, add the datasource template
    if dashboard["templating"]["list"][0]["name"] == "Datasource":
        dashboard["templating"]["list"][0] = PROMETHEUS_DATASOURCE

    # add bigquery datasource
    if contains_string(dashboard, "bigquery"):
        if "templating" not in dashboard:
            dashboard["templating"] = {}
        dashboard["templating"]["list"].insert(1, BIGQUERY_DATASOURCE)

    # replace the name and UID
    dashboard["title"] = f"{GRAFANA_RESOURCE_PREFIX}{dashboard_name}"
    dashboard["uid"] = f"{GRAFANA_RESOURCE_PREFIX}{dashboard_name}"

    if "id" in dashboard:
        dashboard["id"]  # remove the ID so it gets a new one

    with open(f"{GRAFANA_RESOURCE_PREFIX}{dashboard_name}.json", "w") as f:
        json.dump(dashboard, f, indent=2)

    return dashboard  # return the model for manual inspection


if __name__ == "__main__":
    files = os.listdir()
    for file in files:
        if file.endswith(".json") and not file.startswith(GRAFANA_RESOURCE_PREFIX):
            print(f"Migrating {file}")
            dash = migrate_dashboard(file.split(".")[0])

    folder = get_remote_folder(GRAFANA_REMOTE_FOLDER)
    if folder is None:
        raise Exception(f"Folder {GRAFANA_REMOTE_FOLDER} not found")

    files = os.listdir()
    for file in files:
        if file.endswith(".json") and file.startswith(GRAFANA_RESOURCE_PREFIX):
            print(f"Uploading {file}")
            with open(file) as f:
                dash = json.load(f)
            upload_dashboard(dash, folder)
