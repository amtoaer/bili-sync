# 使用多阶段构建
# 第一阶段：Rust编译环境
FROM rustlang/rust:nightly-alpine AS builder

WORKDIR /app

# 配置中国区镜像源
RUN mkdir -p /usr/local/cargo && \
    echo '[source.crates-io]' > /usr/local/cargo/config && \
    echo 'replace-with = "ustc"' >> /usr/local/cargo/config && \
    echo '[source.ustc]' >> /usr/local/cargo/config && \
    echo 'registry = "https://mirrors.ustc.edu.cn/crates.io-index"' >> /usr/local/cargo/config

# 安装编译依赖
RUN apk update && apk add --no-cache \
    build-base \
    musl-dev \
    openssl-dev \
    ca-certificates \
    tzdata \
    git \
    sqlite-dev

# 复制Cargo配置文件和web构建文件
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates/
COPY web ./web/
COPY rustfmt.toml ./

# 创建一个空的主程序，预先构建依赖
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制项目源代码
COPY . .

# 编译项目
RUN cargo build --release && \
    strip target/release/bili-sync-rs

# 第二阶段：运行环境
FROM alpine:latest

WORKDIR /app

# 安装运行时依赖
RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata \
    ffmpeg \
    sqlite-libs

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/bili-sync-rs /app/bili-sync-rs

# 设置权限
RUN chmod +x /app/bili-sync-rs

# 设置环境变量
ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    HOME=/app \
    RUST_BACKTRACE=1 \
    RUST_LOG=None,bili_sync=info

# 指定入口点
ENTRYPOINT [ "/app/bili-sync-rs" ]

# 定义数据卷，用于持久化配置
VOLUME [ "/app/.config/bili-sync" ]

# 暴露Web界面端口（如果有的话）
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD [ "/app/bili-sync-rs", "--health-check" ] || exit 1

# 元数据标签
LABEL maintainer="amtoaer <amtoaer@gmail.com>" \
      description="bili-sync - 由 Rust & Tokio 驱动的哔哩哔哩同步工具" \
      version="2.5.1"

