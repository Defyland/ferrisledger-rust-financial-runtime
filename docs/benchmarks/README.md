# Benchmarks

## Methodology

Native replay is measured with Criterion. HTTP behavior is measured with k6
against the local Axum server.

## Current baseline

The baseline target is correctness-first:

- Smoke: 1 VU, 3 iterations.
- Load: 10 VUs for 1 minute.
- Stress: ramp to 30 VUs.
- Spike: jump to 50 VUs.

Measured output is stored in
[`benchmarks/results/2026-05-30-smoke.md`](../../benchmarks/results/2026-05-30-smoke.md).
The latest smoke result recorded p50 10.54 ms, p95 28.76 ms, p99 30.51 ms,
1.9312 req/s, 0.00% errors, and Criterion replay mean of 292.95 us for 100
deposits.

## Bottlenecks expected

The JSONL store performs full stream reads and fsync on append. That is useful
for the MVP because it exposes durability and replay costs, but it is the first
area to replace with segmented files or PostgreSQL.
