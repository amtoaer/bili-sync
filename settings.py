from dataclasses import dataclass
from dataclasses_json import DataClassJsonMixin
from pathlib import Path
from typing import Self
import os

from constants import DEFAULT_CONFIG_PATH


@dataclass
class Config(DataClassJsonMixin):
    sessdata: str
    bili_jct: str
    buvid3: str
    dedeuserid: str
    ac_time_value: str
    favorite_ids: list[int]
    path_mapper: dict[int, str]

    @staticmethod
    def load(path: Path | None = None) -> Self:
        if not path:
            path = DEFAULT_CONFIG_PATH
        try:
            with path.open("r") as f:
                return Config.schema().loads(f.read())
        except Exception as e:
            raise RuntimeError(f"Failed to load config file: {path}") from e

    def save(self, path: Path | None = None) -> Self:
        if not path:
            path = DEFAULT_CONFIG_PATH
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("w") as f:
                f.write(Config.schema().dumps(self))
            return self
        except Exception as e:
            raise RuntimeError(f"Failed to save config file: {path}") from e


def init_settings() -> Config:
    if DEFAULT_CONFIG_PATH.exists():
        return Config.load(DEFAULT_CONFIG_PATH)
    if os.getenv("TESTING"):
        from debug import debug_config

        return debug_config
    return (
        Config.schema()
        .load(
            {
                "sessdata": "",
                "bili_jct": "",
                "buvid3": "",
                "dedeuserid": "",
                "ac_time_value": "",
                "favorite_ids": [],
                "path_mapper": {},
            }
        )
        .save(DEFAULT_CONFIG_PATH)
    )


settings = init_settings()
