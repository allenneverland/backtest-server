FROM rust:1.87-slim-bullseye

# 安裝基本工具和依賴
RUN apt-get update && apt-get install -y \
    git \
    curl \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    make \
    cmake \
    g++ \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*

# 設置工作目錄
WORKDIR /app

# 安裝Rust開發工具
RUN rustup component add rustfmt clippy

# 設置優化的環境變數
ENV CARGO_HOME=/usr/local/cargo
# 使用 sparse registry (最重要的優化！)
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
# 使用 git CLI 避免記憶體問題
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
# 限制並行編譯任務
ENV CARGO_BUILD_JOBS=4
# 使用 lld 連結器
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld -C target-cpu=native"
ENV RUST_BACKTRACE=1

# 安裝開發工具（使用標準 cargo install）
RUN cargo install cargo-make cargo-edit cargo-watch cargo-llvm-cov sqlx-cli

# 創建 cargo 配置目錄
RUN mkdir -p /usr/local/cargo

# 複製 cargo 配置（如果有的話）
COPY .cargo/config.toml /usr/local/cargo/config.toml