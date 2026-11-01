# Visualization performance evidence

The old table listed timings, FPS, memory and 100k-node results without a reproducible harness or raw measurements. Those figures are withdrawn. They must not be used as product claims or release thresholds. The embedded web renderer and persisted cross-session layout cache mentioned there are not implemented in the shipped UI.

A reproducible **debug smoke test** exists:

```sh
cargo test --locked --no-default-features test_synthetic_1k_graph_performance -- --nocapture
```

It generates a deterministic 1,000-node site with seed 123, builds a CSR snapshot, and computes PageRank. Its broad 500ms assertions are smoke thresholds, sensitive to machine load; they do not establish release performance or UI FPS. `test_layout_precomputation` also checks a layout against synthetic data but does not profile actual GTK rendering.

For release benchmarking, record CPU/OS/toolchain/build profile, graph nodes/edges and seed, multiple-run distributions, peak memory and a profiler trace. Use representative real crawls after removing private content. Measure startup, input responsiveness, search/filter latency, link analysis and serialization separately. Keep raw output next to any published numbers.

This modernization removes per-row full-crawl clones, adds duplicate indexes, throttles aggregate UI refreshes, moves project/Markdown writes off the GTK thread and eliminates repeated stop-word set construction. These are implementation-level cost reductions; no percentage speedup is claimed without before/after measurements.

## Release CPU baseline — 2026-11-01

The [visualization design study](docs/visualization/design.md#measured-baseline) now includes a bounded headless release harness, [raw observations and environment](docs/visualization/baseline.json), and phase-by-phase results. It measures full-result cloning, CSR snapshot construction, rank and one icicle-tree rebuild. The fixture avoids the older synthetic generator's quadratic reverse-link search.

```sh
cargo build --release --locked --no-default-features --example visualization_benchmark
/usr/bin/time -v target/release/examples/visualization_benchmark > /tmp/tadpole-baseline.json
```

Three-trial median icicle rebuilds were 16.05 ms for a bucketed 10k-URL hierarchy, 548.17 ms for 10k URLs in one folder, and 6,035.52 ms for 25k URLs in one folder on the recorded i7-8550U machine. These expose the child-search/layout risk; they do not measure GTK FPS or establish a latency guarantee. The current draw and input handlers rebuild that tree repeatedly. Indexed construction, prepared geometry and cached hit testing are proposed remedies, not implemented speedups in this design change.

The full timed process peaked at 468,660 KiB RSS; this is not incremental visualization memory. Fixtures omit content bodies and several real-crawl metadata fields. The report states excluded phases and sampling limits. Keep the withdrawn historical figures above withdrawn.
