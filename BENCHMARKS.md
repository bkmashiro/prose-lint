# Benchmarks

A local release-mode smoke benchmark was recorded on 2026-08-23.

## Environment

- Apple M4
- 16 GiB RAM
- macOS 26.4.1
- Rust 1.93.1
- `prose-lint` 0.1.0

## Repository-shaped workload

The checked-in generator created 1,000 Markdown files containing 5,328,000
bytes in total. The text intentionally triggers several high-, medium-, and
low-confidence rules in every paragraph, so this measures matching, finding
allocation, sorting, and JSON rendering rather than a clean-text fast path.

```bash
python3 scripts/generate_benchmark_corpus.py /tmp/prose-lint-bench \
  --files 1000 --paragraphs 24
/usr/bin/time -lp target/release/prose-lint \
  scan /tmp/prose-lint-bench --format json >/dev/null
```

Observed result:

```text
real time                 0.34 s
throughput                15.7 MB/s
file rate                 2,941 files/s
maximum resident set      93.5 MiB
release binary             1.75 MiB
```

The peak memory figure is deliberately adversarial: the CLI retains reports in
memory to sort file output deterministically, and this corpus produces many
findings. Ordinary prose produces fewer findings and needs less report memory.

These numbers are a local smoke measurement, not a cross-machine performance
claim. Run the same command on the target machine when performance is material.
