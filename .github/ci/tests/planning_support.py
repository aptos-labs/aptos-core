"""Small manifest graphs and an independent planning oracle."""

from hypothesis import strategies as st


@st.composite
def manifest_graphs(draw):
    variants = [
        {"id": f"variant-{i}", "profile": draw(st.sampled_from(["release", "performance"])),
         "features": draw(st.sampled_from(["", "failpoints", "feature-a,feature-b"])),
         "build_target": draw(st.sampled_from(["all", "validator"]))}
        for i in range(draw(st.integers(1, 4)))
    ]
    ids = [v["id"] for v in variants]
    capabilities = []
    approved = {}
    for i in range(draw(st.integers(1, 5))):
        local = draw(st.lists(st.sampled_from(ids), min_size=1, max_size=len(ids), unique=True))
        publish = draw(st.lists(st.sampled_from(local), max_size=len(local), unique=True))
        workloads = []
        for j in range(draw(st.integers(0, 2))):
            workload = {
                "id": f"workload-{i}-{j}", "docs_sensitive": draw(st.booleans()),
                "image_variant": draw(st.sampled_from(ids)),
                "additional_testing_images": draw(st.booleans()),
                "checks": draw(st.lists(st.sampled_from(["check-a", "check-b", "check-c"]),
                                       min_size=1, max_size=3, unique=True)),
            }
            if draw(st.booleans()):
                workload["marker"] = {"id": f"marker-{i}-{j}",
                    "comment_header": f"header-{i}-{j}", "title": f"Workload {i} {j}"}
            workloads.append(workload)
        cid = f"capability_{i}"
        capabilities.append({"id": cid, "label": f"CICD:capability-{i}",
                             "local": local, "publish": publish, "workloads": workloads})
        approved[cid] = draw(st.booleans())
    return {"version": 1, "image_check": "images-check", "variants": variants,
            "capabilities": capabilities}, approved, draw(st.booleans())


def expected_plan(raw, approvals, docs_only):
    """Apply the security rules directly to the generated raw graph."""
    selected = [c for c in raw["capabilities"] if approvals[c["id"]].approved]
    local_ids = {v for c in selected for v in c["local"]}
    active = {w["id"]: approvals[c["id"]].approved and (not docs_only or not w["docs_sensitive"])
              for c in raw["capabilities"] for w in c["workloads"]}
    active_workloads = [w for c in selected for w in c["workloads"] if active[w["id"]]]
    publish_ids = {v for c in selected for v in c["publish"]} | {w["image_variant"] for w in active_workloads}
    local = [dict(v) for v in raw["variants"] if v["id"] in local_ids]
    publish = [{**v, "additional_testing_images": any(
        w["image_variant"] == v["id"] and w["additional_testing_images"] for w in active_workloads)}
        for v in raw["variants"] if v["id"] in publish_ids]
    markers = [{"marker": w["marker"]["id"], "workload": w["id"]}
               for w in active_workloads if "marker" in w]
    return {
        "docs_only": docs_only,
        "approvals": {cid: {"approved": a.approved, "approver": a.approver,
                            "approval_event_id": a.approval_event_id} for cid, a in approvals.items()},
        "local": {"enabled": bool(local), "include": local},
        "publish": {"enabled": bool(publish), "include": publish},
        "workloads": active, "markers": {"enabled": bool(markers), "include": markers},
    }


def expected_statuses(raw, plan, results):
    workloads = [w for c in raw["capabilities"] for w in c["workloads"]]
    checks = list(dict.fromkeys(check for w in workloads for check in w["checks"]))
    if results["compute-authorization"] != "success":
        return {check: False for check in [raw["image_check"], *checks]}
    wanted_local = "success" if plan["local"]["enabled"] else "skipped"
    wanted_publish = "success" if plan["publish"]["enabled"] else "skipped"
    statuses = {raw["image_check"]: results["pr-rust-images-local"] == wanted_local
                and results["pr-publish-rust-images"] == wanted_publish}
    for check in checks:
        passing = []
        for w in workloads:
            if check in w["checks"]:
                enabled = plan["workloads"][w["id"]]
                passing.append(results[w["id"]] == ("success" if enabled else "skipped")
                               and (not enabled or results["pr-publish-rust-images"] == "success"))
        statuses[check] = all(passing)
    return statuses
