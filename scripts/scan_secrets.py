#!/usr/bin/env python3
"""Fail closed when likely plaintext PAN or credentials enter the repository."""

from __future__ import annotations

import argparse
import re
import subprocess
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Iterable

MAX_TEXT_BYTES = 5 * 1024 * 1024
PAN_PATTERN = re.compile(r"\b[A-Z]{5}[0-9]{4}[A-Z]\b")
API_KEY_PATTERN = re.compile(
    r"\b(?:api[_-]?key|access[_-]?token|secret|token)\b\s*[:=]\s*['\"]?[A-Za-z0-9_./+\-=]{16,}",
    re.IGNORECASE,
)

# Test fixtures must use format-valid synthetic PANs (five letters, four digits,
# one letter) to exercise validation/masking/redaction. These are NOT real PANs.
# We allow that narrowly inside test paths only; every other path fails closed.
_TEST_PATH_MARKERS = ("/tests/", "tests/", "_test.rs", ".test.")


class FindingKind(str, Enum):
    PAN = "PAN"
    API_KEY = "API_KEY"


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    kind: FindingKind


def _scan_text(path: Path, text: str) -> list[Finding]:
    skip_pan = _is_test_path(path)
    findings: list[Finding] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not skip_pan and PAN_PATTERN.search(line):
            findings.append(Finding(path=path, line=line_number, kind=FindingKind.PAN))
        if API_KEY_PATTERN.search(line):
            findings.append(Finding(path=path, line=line_number, kind=FindingKind.API_KEY))
    return findings


def _is_test_path(path: Path) -> bool:
    s = path.as_posix()
    return any(marker in s for marker in _TEST_PATH_MARKERS)


def scan_paths(paths: Iterable[Path]) -> list[Finding]:
    findings: list[Finding] = []
    for path in paths:
        if not path.is_file() or path.stat().st_size > MAX_TEXT_BYTES:
            continue
        data = path.read_bytes()
        if b"\0" in data:
            continue
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            continue
        findings.extend(_scan_text(path, text))
    return findings


def discover_repository_files(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return [root / item.decode("utf-8") for item in result.stdout.split(b"\0") if item]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path)
    args = parser.parse_args()
    root = Path.cwd()
    paths = args.paths or discover_repository_files(root)
    findings = scan_paths(paths)
    for finding in findings:
        try:
            display_path = finding.path.relative_to(root)
        except ValueError:
            display_path = finding.path
        print(f"{display_path}:{finding.line}: blocked {finding.kind.value}")
    if findings:
        print(f"Secret scan failed with {len(findings)} finding(s); values intentionally redacted.")
        return 1
    print(f"Secret scan passed ({len(paths)} files checked).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
