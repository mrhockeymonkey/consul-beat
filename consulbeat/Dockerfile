# Stage 1: Build the Rust binary
FROM rust:latest as builder

WORKDIR /usr/src/consulbeat

# pull dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# compile binary
COPY .. .
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/local/bin

RUN apt update  &&\
    apt install libssl3 &&\
    mkdir /var/log/consul

COPY --from=builder /usr/src/consulbeat/target/release/consulbeat .
CMD ["./consulbeat"]