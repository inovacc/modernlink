#!/usr/bin/env python3
"""Keep PR labeling separate from release creation."""

from pathlib import Path
import re
import sys


WORKFLOW = Path(
    sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/release-drafter.yml"
)
source = WORKFLOW.read_text(encoding="utf-8")

triggers: list[str] = []
in_on_block = False
for line in source.splitlines():
    if line == "on:":
        in_on_block = True
        continue
    if in_on_block and line and not line.startswith(" "):
        break
    trigger_match = re.fullmatch(r"  ([A-Za-z0-9_-]+):", line)
    if in_on_block and trigger_match:
        triggers.append(trigger_match.group(1))

expected_triggers = ["pull_request_target"]
if triggers != expected_triggers:
    sys.exit(
        "release-drafter labeler must run once via pull_request_target; found: "
        + ", ".join(triggers)
    )

actions = re.findall(r"^\s*-\s+uses:\s+(\S+)", source, flags=re.MULTILINE)
expected_action = "release-drafter/release-drafter/autolabeler@v7"
if actions != [expected_action]:
    sys.exit(
        f"release-drafter labeler must use {expected_action}; found: "
        + ", ".join(actions)
    )

print("release-drafter PR workflow uses the dedicated v7 autolabeler")
