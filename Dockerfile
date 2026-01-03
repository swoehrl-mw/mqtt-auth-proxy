FROM clux/muslrust:1.92.0-stable AS builder
RUN mkdir /build
WORKDIR /build
COPY Cargo.toml Cargo.lock /build/
COPY src /build/src
RUN --mount=type=cache,target=/build/target \
    cargo build --release && \
    cp /build/target/x86_64-unknown-linux-musl/release/mqtt-auth-proxy /build/mqtt-auth-proxy

FROM alpine:3.22
WORKDIR /app
COPY --from=builder /build/mqtt-auth-proxy /app/
USER 65532:65532
CMD [ "/app/mqtt-auth-proxy" ]
