# Visualization performance evidence

The old table listed timings, FPS, memory and 100k-node results without a reproducible harness or raw measurements. Those figures are withdrawn. They must not be used as product claims or release thresholds. The embedded web renderer and persisted cross-session layout cache mentioned there are not implemented in the shipped UI.

A reproducible **debug smoke test** exists:

```sh
cargo test --locked --no-default-features test_synthetic_1k_graph_performance -- --nocapture
```

It generates a deterministic 1,000-node site with seed 123, builds a CSR snapshot, and computes PageRank. Its broad 500ms assertions are smoke thresholds, sensitive to machine load; they do not establish release performance or UI FPS. `test_layout_precomputation` also checks a layout against synthetic data but does not profile actual GTK rendering.

For release benchmarking, record CPU/OS/toolchain/build profile, graph nodes/edges and seed, multiple-run distributions, peak memory and a profiler trace. Use representative real crawls after removing private content. Measure startup, input responsiveness, search/filter latency, link analysis and serialization separately. Keep raw output next to any published numbers.

This modernization removes per-row full-crawl clones, adds duplicate indexes, throttles aggregate UI refreshes, moves project/Markdown writes off the GTK thread and eliminates repeated stop-word set construction. These are implementation-level cost reductions; no percentage speedup is claimed without before/after measurements.
