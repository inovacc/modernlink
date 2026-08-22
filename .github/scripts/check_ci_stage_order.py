#!/usr/bin/env python3
"""Check the CI stage contract before any expensive job starts."""

from pathlib import Path
import re
import sys


WORKFLOW = Path(sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/test.yml")
source = WORKFLOW.read_text(encoding="utf-8")


def parse_jobs(text: str) -> dict[str, dict[str, object]]:
    jobs: dict[str, dict[str, object]] = {}
    current: str | None = None
    reading_needs = False
    in_jobs = False
    for line in text.splitlines():
        if line == "jobs:":
            in_jobs = True
            continue
        if in_jobs and line and not line.startswith(" "):
            break
        if not in_jobs:
            continue

        match = re.fullmatch(r"  ([A-Za-z0-9_-]+):", line)
        if match:
            current = match.group(1)
            jobs[current] = {"needs": [], "block": []}
            reading_needs = False
            continue
        if current is None:
            continue
        jobs[current]["block"].append(line)

        match = re.fullmatch(r"    needs:\s*(.*)", line)
        if match:
            value = match.group(1).strip()
            reading_needs = not value
            if value.startswith("[") and value.endswith("]"):
                value = value[1:-1]
            if value:
                jobs[current]["needs"] = [item.strip() for item in value.split(",")]
            continue
        if reading_needs:
            match = re.fullmatch(r"      -\s+([A-Za-z0-9_-]+)", line)
            if match:
                jobs[current]["needs"].append(match.group(1))
            elif line.strip():
                reading_needs = False

    return jobs


jobs = parse_jobs(source)
expected = {
    "rust": [],
    "java6-jar": ["rust"],
    "rust-coverage": ["java6-jar"],
    "broker-backed": ["rust-coverage"],
    "broker-backed-kafka-pulsar": ["rust-coverage"],
    "native-aarch64": ["java6-jar", "rust-coverage"],
    "dependency-audit": ["rust-coverage"],
    "release": [
        "java6-jar",
        "rust-coverage",
        "broker-backed",
        "broker-backed-kafka-pulsar",
        "native-aarch64",
        "dependency-audit",
    ],
}

errors = []
for job, needs in expected.items():
    if job not in jobs:
        errors.append(f"missing staged job: {job}")
        continue
    actual = sorted(jobs[job]["needs"])
    if actual != sorted(needs):
        errors.append(f"{job} needs {actual}, expected {sorted(needs)}")

if "dependency-audit" in jobs:
    audit_block = "\n".join(jobs["dependency-audit"]["block"])
    if "continue-on-error: true" in audit_block:
        errors.append("dependency-audit is non-blocking")

if "release" in jobs:
    release_block = "\n".join(jobs["release"]["block"])
    if "always()" not in release_block:
        errors.append("release gate must evaluate all needs explicitly")
    for dependency in expected["release"]:
        if f"needs['{dependency}'].result == 'success'" not in release_block:
            errors.append(f"release gate does not require {dependency} success")

if errors:
    sys.exit("\n".join(errors))

print("CI stages are ordered Rust -> Java -> coverage -> remaining checks -> release")
