FROM rust:1.95-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p ferrisledger-cli

FROM debian:bookworm-slim

RUN useradd --create-home --uid 10001 ferrisledger \
  && mkdir -p /data \
  && chown -R ferrisledger:ferrisledger /data

COPY --from=builder /app/target/release/ferrisledger /usr/local/bin/ferrisledger

USER ferrisledger
EXPOSE 8080
ENV FERRISLEDGER_STORE_PATH=/data/events.jsonl
ENV FERRISLEDGER_RATE_LIMIT_PER_MINUTE=120
ENV FERRISLEDGER_AUTH_FAILURE_RATE_LIMIT_PER_MINUTE=60

CMD ["ferrisledger", "serve", "--bind", "0.0.0.0:8080"]
