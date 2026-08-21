#!/usr/bin/env python3
"""Require every downstream workflow job to depend transitively on Rust."""

from pathlib import Path
import re
import sys


WORKFLOW = Path(sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/test.yml")
ROOT_JOB = "rust"


def parse_dependencies(source: str) -> dict[str, list[str]]:
    jobs: dict[str, list[str]] = {}
    current_job: str | None = None
    in_jobs = False
    reading_needs = False

    for line in source.splitlines():
        if line == "jobs:":
            in_jobs = True
            continue
        if in_jobs and line and not line.startswith(" "):
            break
        if not in_jobs:
            continue

        job_match = re.fullmatch(r"  ([A-Za-z0-9_-]+):", line)
        if job_match:
            current_job = job_match.group(1)
            jobs[current_job] = []
            reading_needs = False
            continue
        if current_job is None:
            continue

        needs_match = re.fullmatch(r"    needs:\s*(.*)", line)
        if needs_match:
            value = needs_match.group(1).strip()
            reading_needs = not value
            if value.startswith("[") and value.endswith("]"):
                value = value[1:-1]
            if value:
                jobs[current_job].extend(
                    dependency.strip() for dependency in value.split(",")
                )
            continue
        if reading_needs:
            item_match = re.fullmatch(r"      -\s+([A-Za-z0-9_-]+)", line)
            if item_match:
                jobs[current_job].append(item_match.group(1))
            elif line.strip():
                reading_needs = False

    return jobs


def depends_on(
    job: str,
    target: str,
    dependencies: dict[str, list[str]],
    visiting: frozenset[str] = frozenset(),
) -> bool:
    if job in visiting:
        return False
    next_visiting = visiting | {job}
    return any(
        dependency == target
        or depends_on(dependency, target, dependencies, next_visiting)
        for dependency in dependencies.get(job, [])
    )


dependencies = parse_dependencies(WORKFLOW.read_text(encoding="utf-8"))
if ROOT_JOB not in dependencies:
    sys.exit(f'workflow is missing the "{ROOT_JOB}" root job')

ungated_jobs = [
    job
    for job in dependencies
    if job != ROOT_JOB and not depends_on(job, ROOT_JOB, dependencies)
]
if ungated_jobs:
    sys.exit(
        f"jobs without a transitive dependency on {ROOT_JOB}: "
        + ", ".join(ungated_jobs)
    )

print(f"all downstream jobs depend transitively on {ROOT_JOB}")
