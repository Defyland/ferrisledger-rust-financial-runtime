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

Measured output:

- [`benchmarks/results/2026-05-30-smoke.md`](../../benchmarks/results/2026-05-30-smoke.md)
- [`benchmarks/results/2026-05-31-load.md`](../../benchmarks/results/2026-05-31-load.md)

The latest load result recorded p50 15.16 ms, p95 52.47 ms, p99 109.59 ms,
18.959445 req/s, 0.00% errors, server max CPU 10.90%, server max RSS
23,184 KiB, and k6 max RSS 43,408 KiB. The latest Criterion replay point
estimate was 537.04 us for 100 deposits.

## Bottlenecks expected

The JSONL store performs full stream reads and fsync on append. That is useful
for the MVP because it exposes durability and replay costs, but it is the first
area to replace with segmented files or PostgreSQL.
