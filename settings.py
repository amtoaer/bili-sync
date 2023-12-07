from dataclasses import dataclass, field, fields
from pathlib import Path
from typing import Self

from dataclasses_json import DataClassJsonMixin, Undefined

from constants import DEFAULT_CONFIG_PATH


@dataclass
class SubtitleConfig(DataClassJsonMixin):
    dataclass_json_config = {"undefined": Undefined.EXCLUDE}

    font_name: str = "微软雅黑，黑体"  # 字体
    font_size: float = 40  # 字号
    alpha: float = 0.8  # 透明度
    fly_time: float = 5  # 滚动弹幕持续时间
    static_time: float = 10  # 静态弹幕持续时间


@dataclass
class Config(DataClassJsonMixin):
    dataclass_json_config = {"undefined": Undefined.EXCLUDE}

    sessdata: str = ""
    bili_jct: str = ""
    buvid3: str = ""
    dedeuserid: str = ""
    ac_time_value: str = ""
    interval: int = 20
    path_mapper: dict[int, str] = field(default_factory=dict)
    subtitle: SubtitleConfig = field(default_factory=SubtitleConfig)

    def validate(self) -> Self:
        """所有值必须被设置"""
        if not all(getattr(self, f.name) for f in fields(self)):
            raise ValueError("Some config values are not set.")
        return self

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
                f.write(
                    Config.schema().dumps(self, indent=4, ensure_ascii=False)
                )
            return self
        except Exception as e:
            raise RuntimeError(f"Failed to save config file: {path}") from e


def init_settings() -> Config:
    return (
        (
            Config.load(DEFAULT_CONFIG_PATH)
            if DEFAULT_CONFIG_PATH.exists()
            else Config()
        )
        .save(DEFAULT_CONFIG_PATH)
        .validate()
    )


settings = init_settings()
