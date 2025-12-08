"""Create a GitHub release for the latest quarterly revenue PDF."""

import subprocess
import sys

from src.utils.find_latest_pdf import get_latest_pdf, parse_quarter_year


def get_release_tag() -> str:
    """Get the release tag from the latest PDF filename."""
    pdf = get_latest_pdf()
    year, quarter = parse_quarter_year(pdf)
    return f"{year}.Q{quarter}"


def release_exists(tag: str) -> bool:
    """Check if a GitHub release with the given tag already exists."""
    result = subprocess.run(
        ["gh", "release", "view", tag],
        capture_output=True,
    )
    return result.returncode == 0


def create_release(tag: str) -> bool:
    """Create a GitHub release with the given tag."""
    result = subprocess.run(
        [
            "gh",
            "release",
            "create",
            tag,
            "--title",
            f"NVIDIA Quarterly Revenue - {tag}",
            "--notes",
            f"Added NVIDIA quarterly revenue data for {tag}",
            "--latest",
        ],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"Failed to create release: {result.stderr}", file=sys.stderr)
        return False

    return True


def main() -> int:
    """Create a release for the latest quarterly PDF."""
    tag = get_release_tag()
    print(f"Release tag: {tag}")

    if release_exists(tag):
        print(f"Release {tag} already exists, skipping")
        return 0

    if create_release(tag):
        print(f"Created release: {tag}")
        return 0

    return 1


if __name__ == "__main__":
    sys.exit(main())
