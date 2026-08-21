#!/usr/bin/env python3
"""Fail when a JaCoCo XML report's aggregate line ratio is below a threshold."""

from __future__ import print_function

import sys
import xml.etree.ElementTree as ElementTree


def main(argv):
    if len(argv) != 3:
        print("usage: check_jacoco_line_coverage.py REPORT.xml MINIMUM", file=sys.stderr)
        return 2

    report_path = argv[1]
    minimum = float(argv[2])
    root = ElementTree.parse(report_path).getroot()
    line_counter = None
    for counter in root.findall("counter"):
        if counter.get("type") == "LINE":
            line_counter = counter
            break
    if line_counter is None:
        print("JaCoCo report has no aggregate LINE counter", file=sys.stderr)
        return 2

    covered = int(line_counter.get("covered", "0"))
    missed = int(line_counter.get("missed", "0"))
    total = covered + missed
    ratio = float(covered) / total if total else 0.0
    print("Java line coverage: {0:.2%} ({1}/{2}); minimum: {3:.2%}".format(
        ratio, covered, total, minimum
    ))
    return 0 if ratio >= minimum else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
