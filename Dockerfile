FROM rust:1.74-slim AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock .
COPY src ./src
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine
WORKDIR /usr/local/bin
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/wsmsg .
ENTRYPOINT ["wsmsg"]