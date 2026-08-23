#!/usr/bin/env python3
"""Generate a deterministic prose corpus for local throughput checks."""

from pathlib import Path
import argparse

PARAGRAPH = (
    "The worker reads one configuration file and validates each entry before use. "
    "Importantly, this is not just a parser, but a robust execution boundary. "
    "The policy lives in the host layer. This ensures predictable behavior.\n\n"
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--files", type=int, default=1000)
    parser.add_argument("--paragraphs", type=int, default=24)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    content = PARAGRAPH * args.paragraphs
    for index in range(args.files):
        (args.output / f"fixture-{index:05}.md").write_text(content, encoding="utf-8")
    total = len(content.encode()) * args.files
    print(f"wrote {args.files} files and {total} bytes")


if __name__ == "__main__":
    main()
