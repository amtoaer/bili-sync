FROM alpine:3.20 AS base

ARG TARGETPLATFORM

WORKDIR /app

RUN set -eux; \
    sed -i 's|https://dl-cdn.alpinelinux.org/alpine|https://mirrors.aliyun.com/alpine|g' /etc/apk/repositories; \
    success=0; \
    for i in 1 2 3 4 5; do \
      if apk add --no-cache ca-certificates tzdata ffmpeg python3 py3-pip nodejs npm; then success=1; break; fi; \
      echo "apk install failed (attempt ${i}), retrying..."; \
      sleep $((i * 3)); \
    done; \
    test "$success" -eq 1

RUN set -eux; \
    success=0; \
    for i in 1 2 3 4 5; do \
      if python3 -m pip install --no-cache-dir --break-system-packages --upgrade requests yt-dlp yt-dlp-ejs; then success=1; break; fi; \
      echo "pip install failed (attempt ${i}), retrying..."; \
      sleep $((i * 5)); \
    done; \
    test "$success" -eq 1

COPY ./bili-sync-rs-Linux-*.tar.gz  ./targets/

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
    tar xzvf ./targets/bili-sync-rs-Linux-x86_64-musl.tar.gz  -C ./; \
    elif [ "$TARGETPLATFORM" = "linux/arm/v7" ]; then \
    tar xzvf ./targets/bili-sync-rs-Linux-armv7-musl.tar.gz  -C ./; \
    else \
    tar xzvf ./targets/bili-sync-rs-Linux-aarch64-musl.tar.gz  -C ./; \
    fi

RUN rm -rf ./targets && chmod +x ./bili-sync-rs && mkdir -p /download

FROM scratch

WORKDIR /app

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    HOME=/app \
    BILI_SYNC_IN_CONTAINER=1 \
    RUST_BACKTRACE=1 \
    RUST_LOG=None,bili_sync=info

COPY --from=base / /

ENTRYPOINT [ "/app/bili-sync-rs" ]

VOLUME [ "/app/.config/bili-sync", "/download" ]
