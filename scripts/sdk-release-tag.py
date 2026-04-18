#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

if sys.version_info < (3, 11):
    raise SystemExit("ERROR: Python 3.11+ is required (tomllib is unavailable on older Python versions).")

import tomllib


REPO_ROOT = Path(__file__).resolve().parents[1]
SDK_CARGO_TOML = REPO_ROOT / "sdk" / "Cargo.toml"
SDK_CRATE_NAME = "kagi-sdk"
RELEASE_VERSION_PATTERN = re.compile(r"^\d+\.\d+\.\d+$")
SDK_TAG_PATTERN = re.compile(r"^sdk-v\d+\.\d+\.\d+$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="scripts/sdk-release-tag.py",
        description=(
            "Create and push an SDK release tag in the format sdk-vX.Y.Z.\n"
            "This helper does not publish crates; it only manages tags."
        ),
        epilog=(
            "Examples:\n"
            "  scripts/sdk-release-tag.py --check\n"
            "  scripts/sdk-release-tag.py\n"
            "  scripts/sdk-release-tag.py --force\n\n"
            "Run this only when the intended SDK release snapshot is exactly the current\n"
            "HEAD on origin/main.\n\n"
            "--force is intentionally narrow: it is only accepted when the local sdk-vX.Y.Z\n"
            "tag already exists at HEAD and origin is missing that same tag (safe push retry)."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Run safety checks and print intended action without creating/pushing tags.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Retry pushing an already-correct existing local SDK tag when remote tag is absent.",
    )
    return parser.parse_args()


def fail(message: str) -> "Never":
    raise SystemExit(f"ERROR: {message}")


def run_git(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    process = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
    )

    if check and process.returncode != 0:
        stderr = process.stderr.strip()
        quoted = " ".join(args)
        fail(f"git {quoted} failed ({process.returncode}): {stderr or 'no stderr output'}")

    return process


def load_toml(path: Path) -> dict:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"Required file is missing: {path}")
    except tomllib.TOMLDecodeError as exc:
        fail(f"Invalid TOML in {path}: {exc}")


def resolve_sdk_release() -> tuple[str, str]:
    sdk_data = load_toml(SDK_CARGO_TOML)
    package_table = sdk_data.get("package", {})
    package_name = package_table.get("name")

    if package_name != SDK_CRATE_NAME:
        fail(
            f"sdk/Cargo.toml package.name must be '{SDK_CRATE_NAME}' (found {package_name!r})."
        )

    sdk_version = package_table.get("version")
    if not isinstance(sdk_version, str) or not RELEASE_VERSION_PATTERN.fullmatch(sdk_version):
        fail(
            "sdk/Cargo.toml package.version must be an explicit release semver in X.Y.Z form "
            f"(found {sdk_version!r})."
        )

    tag_name = f"sdk-v{sdk_version}"
    if not SDK_TAG_PATTERN.fullmatch(tag_name):
        fail(
            "Derived SDK release tag does not match required grammar sdk-vX.Y.Z "
            f"(derived {tag_name!r})."
        )

    return sdk_version, tag_name


def ensure_head_matches_origin_main() -> str:
    print("Refreshing origin/main...")
    run_git("fetch", "--no-tags", "--prune", "origin", "main")

    head_sha = run_git("rev-parse", "HEAD").stdout.strip()
    origin_main_sha = run_git("rev-parse", "origin/main").stdout.strip()

    if head_sha != origin_main_sha:
        fail(
            "HEAD must exactly match origin/main before tagging. "
            f"HEAD={head_sha[:12]} origin/main={origin_main_sha[:12]}"
        )

    return head_sha


def ensure_clean_worktree_and_index() -> None:
    status = run_git("status", "--porcelain", "--untracked-files=all").stdout.strip()
    if status:
        fail(
            "Working tree and index must be completely clean before SDK tag create/push. "
            "Commit, stash, or discard all changes (including untracked files) first."
        )


def get_local_tag_sha(tag_name: str) -> str | None:
    result = run_git("show-ref", "--tags", "--verify", f"refs/tags/{tag_name}", check=False)
    if result.returncode != 0:
        return None
    return run_git("rev-list", "-n", "1", tag_name).stdout.strip()


def get_remote_tag_target_commit(tag_name: str) -> str | None:
    result = run_git(
        "ls-remote",
        "--tags",
        "origin",
        f"refs/tags/{tag_name}",
        f"refs/tags/{tag_name}^{{}}",
    )
    lines = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    if not lines:
        return None

    refs_to_sha: dict[str, str] = {}
    for line in lines:
        parts = line.split()
        if len(parts) != 2:
            fail(f"Unexpected ls-remote output line for tag {tag_name!r}: {line!r}")
        sha, ref = parts
        refs_to_sha[ref] = sha

    peeled_ref = f"refs/tags/{tag_name}^{{}}"
    direct_ref = f"refs/tags/{tag_name}"

    if peeled_ref in refs_to_sha:
        return refs_to_sha[peeled_ref]
    if direct_ref in refs_to_sha:
        return refs_to_sha[direct_ref]

    fail(f"Could not resolve remote commit target for tag {tag_name!r}.")


def plan_tag_action(tag_name: str, head_sha: str) -> str:
    local_tag_sha = get_local_tag_sha(tag_name)
    remote_tag_sha = get_remote_tag_target_commit(tag_name)

    if remote_tag_sha is not None:
        if remote_tag_sha != head_sha:
            fail(
                f"Remote tag {tag_name} already exists at {remote_tag_sha[:12]}, not HEAD {head_sha[:12]}. "
                "Refusing to rewrite SDK semver tags."
            )
        if local_tag_sha is not None and local_tag_sha != remote_tag_sha:
            fail(
                f"Local tag {tag_name} points to {local_tag_sha[:12]} but remote points to "
                f"{remote_tag_sha[:12]}. Refusing to rewrite SDK semver tags."
            )
        return "noop"

    if local_tag_sha is not None:
        if local_tag_sha != head_sha:
            fail(
                f"Local tag {tag_name} already exists at {local_tag_sha[:12]}, not HEAD {head_sha[:12]}. "
                "Refusing to rewrite SDK semver tags."
            )
        return "push-existing"

    return "create-and-push"


def ensure_force_policy(action: str, force: bool) -> None:
    if action == "push-existing":
        if force:
            return
        fail(
            "Planned action is push-existing, which is intentionally gated. "
            "Re-run with --force to retry pushing the existing correct local tag."
        )

    if force:
        fail(
            "--force is only allowed when retrying a push of an existing local SDK tag "
            "that already points to HEAD while origin is missing it."
        )


def ensure_head_stable(expected_head_sha: str) -> None:
    current_head_sha = run_git("rev-parse", "HEAD").stdout.strip()
    if current_head_sha != expected_head_sha:
        fail(
            "HEAD changed during checks; aborting to avoid tagging an unexpected snapshot. "
            f"expected {expected_head_sha[:12]}, current {current_head_sha[:12]}"
        )


def execute_tag_action(tag_name: str, head_sha: str, action: str) -> None:
    if action == "noop":
        print(f"Tag {tag_name} is already present on origin at HEAD {head_sha[:12]}; nothing to do.")
        return

    if action == "push-existing":
        print(f"Pushing existing local tag {tag_name} to origin...")
        run_git("push", "origin", f"refs/tags/{tag_name}:refs/tags/{tag_name}")
        print(f"Pushed existing tag {tag_name}.")
        return

    if action == "create-and-push":
        print(f"Creating local tag {tag_name} at HEAD {head_sha[:12]}...")
        run_git("tag", tag_name, head_sha)
        print(f"Pushing tag {tag_name} to origin...")
        run_git("push", "origin", f"refs/tags/{tag_name}:refs/tags/{tag_name}")
        print(f"Created and pushed tag {tag_name}.")
        return

    fail(f"Unsupported action: {action}")


def main() -> int:
    args = parse_args()

    version, tag_name = resolve_sdk_release()
    print(f"Resolved SDK release version: {version}")
    print(f"Derived SDK tag: {tag_name}")

    head_sha = ensure_head_matches_origin_main()
    action = plan_tag_action(tag_name, head_sha)
    print(f"Planned action: {action}")
    ensure_force_policy(action, args.force)

    if action in {"create-and-push", "push-existing"}:
        print("Checking provenance guard: clean working tree/index at current HEAD snapshot...")
        ensure_clean_worktree_and_index()
        ensure_head_stable(head_sha)

    if args.check:
        print("Check mode: PASS")
        return 0

    execute_tag_action(tag_name, head_sha, action)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
