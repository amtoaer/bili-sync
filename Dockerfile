FROM python:3.11.6-alpine3.18 AS base

WORKDIR /app

ENV BILI_IN_DOCKER=true

COPY poetry.lock pyproject.toml ./

RUN apk add ffmpeg \
    && apk add --no-cache --virtual .build-deps \
    gcc \
    musl-dev \
    libffi-dev \
    openssl-dev \
    && pip install poetry \
    && poetry config virtualenvs.create false \
    && poetry install --no-dev --no-interaction --no-ansi \
    && apk del .build-deps

COPY . .

ENTRYPOINT [ "python", "entry.py" ]