import functions_framework
import os
from google.cloud import storage
from datetime import timedelta
from google import auth
from google.auth.transport import requests


@functions_framework.http
def handler(request):
    print(request)
    namespace = request.args.get("namespace")
    era = request.args.get("era")
    method = request.args.get("method", "GET")
    cluster_name = request.args.get("cluster_name")
    bucket_name = os.environ["BUCKET_NAME"]

    credentials, _ = auth.default()
    if credentials.token is None:
        credentials.refresh(requests.Request())

    storage_client = storage.Client()
    bucket = storage_client.bucket(bucket_name)
    blob = bucket.blob(f"{cluster_name}/{namespace}/{era}/genesis.blob")

    url = blob.generate_signed_url(
        version="v4",
        method=method,
        expiration=timedelta(hours=1),
        service_account_email=credentials.service_account_email,
        access_token=credentials.token,
    )
    return url
