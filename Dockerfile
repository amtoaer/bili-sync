FROM python:3.11.6-alpine3.19

WORKDIR /app

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    BILI_IN_DOCKER=true

COPY poetry.lock pyproject.toml ./

RUN apk add --no-cache ffmpeg tini \
    && apk add --no-cache --virtual .build-deps \
        gcc \
        musl-dev \
        libffi-dev \
        openssl-dev \
    && pip install poetry \
    && poetry config virtualenvs.create false \
    && poetry install --no-dev --no-interaction --no-ansi \
    && apk del .build-deps \
    && rm -rf \
        /root/.cache \
        /tmp/*

COPY . .

ENTRYPOINT [ "tini", "python", "entry.py" ]

VOLUME [ "/app/config", "/app/data", "/app/thumb", "/Videos/Bilibilis" ]