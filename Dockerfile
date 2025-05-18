FROM rust:1.86-slim-bullseye

# 安裝基本工具和依賴
RUN apt-get update && apt-get install -y \
    git \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    make \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# 設置工作目錄
WORKDIR /app

# 安裝Rust開發工具
RUN rustup component add rustfmt clippy
RUN cargo install cargo-make cargo-edit cargo-watch cargo-llvm-cov sqlx-cli

# 預先設置Cargo環境變量以提高構建性能
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTFLAGS="-C target-cpu=native"
ENV RUST_BACKTRACE=1
