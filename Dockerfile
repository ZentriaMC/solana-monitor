# docker build --load -t localhost/zentria/solana-monitor:latest .
ARG rust_version="1.92.0"

FROM rust:${rust_version}-alpine AS rust-base

FROM rust:${rust_version}-alpine AS builder

WORKDIR /build

ENV CARGO_INCREMENTAL="0"

RUN --mount=source=.,target=. \
    --mount=type=cache,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=locked,from=rust-base,source=/usr/local/rustup,target=/usr/local/rustup \
    cargo fetch --locked

RUN --mount=source=.,target=. \
    --mount=type=cache,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=locked,from=rust-base,source=/usr/local/rustup,target=/usr/local/rustup \
    --mount=type=cache,sharing=locked,target=/target \
    --network=none <<-EOF
arch="$(uname -m)"
case "${arch}" in
    x86_64)
        CARGO_BUILD_TARGET="x86_64-unknown-linux-musl"
        ;;
    aarch64)
        CARGO_BUILD_TARGET="aarch64-unknown-linux-musl"
        ;;
    *) echo "Unsupported architecture: ${arch}" >&2; exit 1 ;;
esac

export CARGO_BUILD_TARGET
cargo build --release --target-dir=/target
EOF

RUN --mount=type=cache,sharing=locked,target=/target,ro \
    mkdir -p /build && \
    cp /target/$(uname -m)-*-musl/release/solana-monitor /build/solana-monitor

FROM scratch

COPY --from=builder --chown=0:0 /build/solana-monitor /usr/local/bin/solana-monitor

USER 2000:2000
ENV PATH=/usr/local/bin

CMD ["solana-monitor"]
