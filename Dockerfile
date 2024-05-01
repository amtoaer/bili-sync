FROM alpine as base

ARG TARGETPLATFORM

WORKDIR /app

RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata \
    ffmpeg

COPY ./*-bili-sync-rs ./targets/

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
    mv ./targets/Linux-x86_64-bili-sync-rs ./bili-sync-rs; \
    else \
    mv ./targets/Linux-aarch64-bili-sync-rs ./bili-sync-rs; \
    fi

RUN rm -rf ./targets && chmod +x ./bili-sync-rs

FROM scratch

WORKDIR /app

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    HOME=/app \
    RUST_BACKTRACE=1 \
    RUST_LOG=None,bili_sync=info

COPY --from=base / /

ENTRYPOINT [ "/app/bili-sync-rs" ]

VOLUME [ "/app/.config/bili-sync" ]

