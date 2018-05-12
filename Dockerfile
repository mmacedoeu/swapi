FROM alpine:edge

RUN apk add --no-cache \
  g++ \
  bash \
  openssl \
  openssl-dev \
  openssl-dbg \
  openssh \
  make \
  wget \
  strace \
  musl-utils \
  git && rm -rf /var/cache/apk/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN set -eux; \
    \
    url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain nightly; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;    

WORKDIR /source
