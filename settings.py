from dataclasses import dataclass
from dataclasses_json import DataClassJsonMixin
from pathlib import Path
from typing import Self


@dataclass
class Config(DataClassJsonMixin):
    sessiondata: str
    bili_jct: str
    buvid3: str
    dedeuserid: str
    ac_time_value: str
    favorite_ids: list
    path_mapper: dict

    @staticmethod
    def load(path: Path | str) -> Self:
        if isinstance(path, str):
            path = Path(path)
        try:
            with path.open("r") as f:
                return Config.schema().loads(f.read())
        except Exception as e:
            raise ValueError(f"Failed to load config file: {path}") from e

    def save(self, path: Path | str) -> Self:
        if isinstance(path, str):
            path = Path(path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("w") as f:
                f.write(Config.schema().dumps(self))
        except Exception as e:
            raise PermissionError(f"Failed to save config file: {path}") from e
        return self


def init_settings() -> Config:
    if (Path(__file__).parent / "config.json").exists():
        return Config.load(Path(__file__).parent / "config.json")
    # TODO: 读取环境变量
    return Config().save(Path(__file__).parent / "config.json")


settings = init_settings()
