FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev libasound2-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/edge_agent*

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y ca-certificates libssl3 libasound2 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/edge-agent /app/edge-agent
COPY --from=builder /app/assets /app/assets

ENV RUST_LOG=info

#todo
ENV HUB_URL=http://hub-java:8080
ENV DEVICE_ID=docker-agent-01
ENV LOOP_INTERVAL_SECS=5

CMD ["./edge-agent"]