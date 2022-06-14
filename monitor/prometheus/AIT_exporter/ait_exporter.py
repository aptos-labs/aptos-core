#!/usr/bin/env python3
import time
import logging

import requests
import prometheus_client

from lxml import html


logging.basicConfig(level=logging.DEBUG,
                    format='%(asctime)s %(filename)s[line:%(lineno)d] %(levelname)s %(message)s',
                    datefmt='%a, %d %b %Y %H:%M:%S',
                    filemode='a')

liveness_metric = prometheus_client.Gauge("aptos_validator_liveness", "", ["peer_id"])
participation_metric = prometheus_client.Gauge("aptos_validator_participation", "", ["peer_id"])
get_data_error_metrics = prometheus_client.Gauge("aptos_validator_get_data_error", "")

logger = logging.getLogger("aptos_exporter")


def get_data():
    data = []
    try:
        response = requests.get("https://community.aptoslabs.com/it1", timeout=30)
    except Exception as exc:
        logger.warning("requests.get error exc: {exc}")
        return data

    if response.status_code != 200:
        logger.warning(f"response.status_code != 200 status_code: {response.status_code} body: {response.text}")
        return data

    tree = html.fromstring(response.content)
    tr_list = tree.xpath("/html/body/div/main/div/div/turbo-frame/div/table/tbody/tr")
    for item in tr_list:
        peer_id = item.getchildren()[1].text.strip("\n").strip(" ")

        liveness_content = item.getchildren()[2].text_content()
        liveness = liveness_content.split("%")[0].rsplit(" ", 1)[1]
        liveness = float(liveness)

        participation_content = item.getchildren()[3].text_content()
        participation = participation_content.split("%")[0].rsplit(" ", 1)[1]
        participation = float(participation)

        data_item = {
            "peer_id": peer_id,
            "liveness": liveness,
            "participation": participation,
        }
        data.append(data_item)

    return data


def update_metrics():
    try:
        item_list = get_data()
    except Exception as exc:
        get_data_error_metrics.set(1)
        logger.warning(f"get_data error exc: {exc}")

    for item in item_list:
        liveness_metric.labels(item["peer_id"]).set(item["liveness"])
        participation_metric.labels(item["peer_id"]).set(item["participation"])
        get_data_error_metrics.set(0)

    logger.info("update metrics successful")


if __name__ == '__main__':
    prometheus_client.start_http_server(9116)
    times = 0
    logger.info("started")
    while True:
        times += 1
        logger.info(f"starting times {times}")
        try:
            update_metrics()
        except Exception as exc:
            get_data_error_metrics.set(1)
            logger.warning(f"update_metrics error exc: {exc}")

        time.sleep(60 * 2)
