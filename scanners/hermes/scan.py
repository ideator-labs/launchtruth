import json
import sys

import osv_check_vendored as hermes


def scan(command, args):
    ecosystem = hermes._infer_ecosystem(command)
    if not ecosystem:
        return None
    name, _version = hermes._parse_package_from_args(args, ecosystem)
    if not name:
        return None
    return {"ecosystem": ecosystem, "name": name}


for case in json.load(sys.stdin):
    verdict = {"id": case["id"], "scanner": "hermes", "package": scan(case["command"], case["args"])}
    print(json.dumps(verdict))
