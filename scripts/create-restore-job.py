#!/usr/bin/env python

import json, random, string, sys

template = json.loads(sys.stdin.read())
if "items" in template:
  template = next(item for item in template["items"] if item["spec"]["completions"] == 0)

random_id = "".join(random.choice(string.ascii_lowercase + string.digits) for i in range(4))
template["metadata"]["name"] += "-" + random_id
template["spec"]["completions"] = 1
# if given an extra argument, take it as pvc name
if len(sys.argv) == 2:
  for volume in template["spec"]["template"]["spec"]["volumes"]:
    if volume["name"] == "velor-data":
      volume["persistentVolumeClaim"] = {"claimName": sys.argv[1]}
del template["spec"]["selector"]["matchLabels"]["controller-uid"]
del template["spec"]["template"]["metadata"]["labels"]["controller-uid"]
del template["spec"]["template"]["metadata"]["labels"]["job-name"]

print(json.dumps(template, indent=4))

