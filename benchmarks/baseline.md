# Benchmark Baseline

## Scope

FerrisLedger measures two paths:

1. Native replay benchmark with Criterion: `replay_100_deposits`.
2. HTTP scenarios with k6: smoke, load, stress, and spike.

## Commands

```bash
cargo bench -p ferrisledger-runtime --bench replay
k6 run benchmarks/k6-smoke.js
k6 run benchmarks/k6-load.js
k6 run benchmarks/k6-stress.js
k6 run benchmarks/k6-spike.js
```

## Metrics to record

- p50, p95, p99 latency
- throughput
- error rate
- CPU and memory notes
- bottleneck and next optimization
