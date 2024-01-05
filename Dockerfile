FROM python:3.11.7-alpine3.19 as base

WORKDIR /app

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    BILI_IN_DOCKER=true

RUN apk add --no-cache ffmpeg tini \
    && apk add --no-cache --virtual .build-deps \
        gcc \
        musl-dev \
        libffi-dev \
        openssl-dev \
    && pip install poetry==1.7.1 pip3-autoremove==1.2.0

COPY poetry.lock pyproject.toml ./

RUN poetry config virtualenvs.create false \
    && poetry install --only main --no-root \
    && pip3-autoremove -y poetry pip3-autoremove \
    && apk del .build-deps \
    && rm -rf \
        /root/.cache \
        /tmp/*

COPY . .

FROM scratch

WORKDIR /app

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    BILI_IN_DOCKER=true

COPY --from=base / /

ENTRYPOINT [ "tini", "python", "entry.py" ]

VOLUME [ "/app/config", "/app/data", "/app/thumb", "/Videos/Bilibilis" ]