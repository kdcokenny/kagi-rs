#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

if sys.version_info < (3, 11):
    raise SystemExit("ERROR: Python 3.11+ is required (tomllib is unavailable on older Python versions).")

import tomllib


SEMVER_PATTERN = re.compile(r"^\d+\.\d+\.\d+$")


@dataclass(frozen=True)
class PublishDecision:
    publish_state: str
    should_run_publish: bool
    version: str
    version_exists_on_crates_io: bool
    reason: str

    def as_github_outputs(self) -> dict[str, str]:
        return {
            "publish_state": self.publish_state,
            "should_run_publish": "true" if self.should_run_publish else "false",
            "version": self.version,
            "version_exists_on_crates_io": "true" if self.version_exists_on_crates_io else "false",
            "decision_reason": self.reason,
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="scripts/release-publish-decision.py",
        description=(
            "Evaluate release publish state from tag/version invariants, crates.io version state, "
            "and GitHub run_attempt."
        ),
    )
    parser.add_argument("--crate-name", required=True, help="Crate name on crates.io (for example: kagi-sdk)")
    parser.add_argument("--manifest-path", required=True, help="Cargo.toml path for the release crate")
    parser.add_argument("--tag-prefix", required=True, help="Required tag prefix before semver (for example: sdk-v)")
    parser.add_argument("--tag-name", required=True, help="Git tag name that triggered the workflow")
    parser.add_argument("--run-attempt", required=True, help="GitHub run attempt number (1 for first run)")
    parser.add_argument("--repository", required=True, help="Actual repository from GITHUB_REPOSITORY")
    parser.add_argument("--expected-repository", required=True, help="Canonical repository allowed to publish")
    parser.add_argument(
        "--version-exists-override",
        choices=("true", "false"),
        help="Testing override for crates.io version existence checks.",
    )
    parser.add_argument("--github-output", required=True, help="Path to GITHUB_OUTPUT")
    return parser.parse_args()


def conflict(reason: str, version: str = "") -> PublishDecision:
    return PublishDecision(
        publish_state="conflict",
        should_run_publish=False,
        version=version,
        version_exists_on_crates_io=False,
        reason=reason,
    )


def append_github_output(output_path: Path, key: str, value: str) -> None:
    with output_path.open("a", encoding="utf-8") as handle:
        handle.write(f"{key}={value}\n")


def parse_tag_version(tag_prefix: str, tag_name: str) -> str | None:
    pattern = re.compile(re.escape(tag_prefix) + r"(\d+\.\d+\.\d+)")
    match = pattern.fullmatch(tag_name)
    if match is None:
        return None
    return match.group(1)


def load_manifest(path: Path) -> dict:
    try:
        raw = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise RuntimeError(f"Required manifest is missing: {path}") from None

    try:
        return tomllib.loads(raw)
    except tomllib.TOMLDecodeError as exc:
        raise RuntimeError(f"Invalid TOML in {path}: {exc}") from exc


def parse_run_attempt(value: str) -> int | None:
    try:
        run_attempt = int(value)
    except ValueError:
        return None
    if run_attempt < 1:
        return None
    return run_attempt


def version_exists_on_crates_io(crate_name: str, version: str) -> tuple[bool | None, str | None]:
    url = f"https://crates.io/api/v1/crates/{crate_name}/{version}"
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            payload = json.load(response)
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            return False, None
        return None, f"::error::crates.io request failed with HTTP {exc.code} for {url}."
    except urllib.error.URLError as exc:
        return None, f"::error::Unable to reach crates.io while checking {crate_name}@{version} ({exc})."
    except json.JSONDecodeError as exc:
        return None, f"::error::crates.io response was not valid JSON for {url} ({exc})."

    reported_version = payload.get("version", {}).get("num")
    if reported_version == version:
        return True, None

    return (
        None,
        "::error::crates.io returned an unexpected payload while checking existing version "
        f"({reported_version!r} for expected {version!r}).",
    )


def decide_publish_state(run_attempt: int, version_exists: bool, version: str) -> PublishDecision:
    if run_attempt == 1 and version_exists:
        return PublishDecision(
            publish_state="conflict",
            should_run_publish=False,
            version=version,
            version_exists_on_crates_io=True,
            reason=(
                f"::error::{version} is already published on crates.io during a fresh run. "
                "Refusing duplicate publish."
            ),
        )

    if run_attempt > 1 and version_exists:
        return PublishDecision(
            publish_state="recovered_after_publish",
            should_run_publish=False,
            version=version,
            version_exists_on_crates_io=True,
            reason="Publish already completed in an earlier run attempt; skipping duplicate publish.",
        )

    if run_attempt > 1:
        return PublishDecision(
            publish_state="published_on_retry",
            should_run_publish=True,
            version=version,
            version_exists_on_crates_io=False,
            reason="Crate version not found on retry attempt; rerunning full publish path.",
        )

    return PublishDecision(
        publish_state="published_on_first_attempt",
        should_run_publish=True,
        version=version,
        version_exists_on_crates_io=False,
        reason="Fresh attempt with unpublished version; running full publish path.",
    )


def evaluate(args: argparse.Namespace) -> PublishDecision:
    if args.repository != args.expected_repository:
        return conflict(
            "::error::This workflow only publishes from "
            f"{args.expected_repository!r}, but received repository {args.repository!r}."
        )

    tag_version = parse_tag_version(args.tag_prefix, args.tag_name)
    if tag_version is None:
        return conflict(
            f"::error::Release tag {args.tag_name!r} must match {args.tag_prefix}X.Y.Z."
        )

    manifest_path = Path(args.manifest_path)
    try:
        manifest = load_manifest(manifest_path)
    except RuntimeError as exc:
        return conflict(f"::error::{exc}", version=tag_version)

    package = manifest.get("package", {})
    crate_name = package.get("name")
    if crate_name != args.crate_name:
        return conflict(
            f"::error::{manifest_path} package.name must be {args.crate_name!r} "
            f"(found {crate_name!r}).",
            version=tag_version,
        )

    manifest_version = package.get("version")
    if not isinstance(manifest_version, str) or not SEMVER_PATTERN.fullmatch(manifest_version):
        return conflict(
            f"::error::{manifest_path} package.version must be an explicit X.Y.Z string "
            f"(found {manifest_version!r}).",
            version=tag_version,
        )

    if manifest_version != tag_version:
        return conflict(
            "::error::Release tag version does not match manifest package.version "
            f"({tag_version} != {manifest_version}).",
            version=tag_version,
        )

    run_attempt = parse_run_attempt(args.run_attempt)
    if run_attempt is None:
        return conflict(
            f"::error::GITHUB_RUN_ATTEMPT must be an integer >= 1 (found {args.run_attempt!r}).",
            version=manifest_version,
        )

    if args.version_exists_override is not None:
        exists_on_crates_io = args.version_exists_override == "true"
        error_message = None
    else:
        exists_on_crates_io, error_message = version_exists_on_crates_io(args.crate_name, manifest_version)

    if error_message is not None or exists_on_crates_io is None:
        return conflict(error_message or "::error::Unknown crates.io state.", version=manifest_version)

    return decide_publish_state(run_attempt, exists_on_crates_io, manifest_version)


def main() -> int:
    args = parse_args()
    output_path = Path(args.github_output)

    decision = evaluate(args)
    for key, value in decision.as_github_outputs().items():
        append_github_output(output_path, key, value)

    print(f"publish_state={decision.publish_state}")
    print(f"should_run_publish={decision.should_run_publish}")
    print(f"version={decision.version}")
    print(f"version_exists_on_crates_io={decision.version_exists_on_crates_io}")
    print(f"decision_reason={decision.reason}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
