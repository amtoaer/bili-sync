from pathlib import Path

from bilibili_api.video import VideoCodecs
from pydantic import BaseModel, Field, field_validator
from pydantic_core import PydanticCustomError
from typing_extensions import Annotated

from constants import DEFAULT_CONFIG_PATH


class SubtitleConfig(BaseModel):
    font_name: str = "微软雅黑，黑体"  # 字体
    font_size: float = 40  # 字号
    alpha: float = 0.8  # 透明度
    fly_time: float = 5  # 滚动弹幕持续时间
    static_time: float = 10  # 静态弹幕持续时间


class Config(BaseModel):
    sessdata: Annotated[str, Field(min_length=1)] = ""
    bili_jct: Annotated[str, Field(min_length=1)] = ""
    buvid3: Annotated[str, Field(min_length=1)] = ""
    dedeuserid: Annotated[str, Field(min_length=1)] = ""
    ac_time_value: Annotated[str, Field(min_length=1)] = ""
    interval: int = 20
    path_mapper: dict[int, str] = Field(default_factory=dict)
    subtitle: SubtitleConfig = Field(default_factory=SubtitleConfig)
    codec: list[VideoCodecs] = Field(
        default_factory=lambda: [VideoCodecs.AV1, VideoCodecs.AVC, VideoCodecs.HEV], min_length=1
    )
    paginated_video: bool = False

    @field_validator("codec", mode="after")
    def codec_validator(cls, codecs: list[VideoCodecs]) -> list[VideoCodecs]:
        if len(codecs) != len(set(codecs)):
            raise PydanticCustomError("unique_list", "List must be unique")
        return codecs

    @staticmethod
    def load(path: Path | None = None) -> "Config":
        if not path:
            path = DEFAULT_CONFIG_PATH
        try:
            with path.open("r") as f:
                return Config.model_validate_json(f.read())
        except Exception as e:
            raise RuntimeError(f"Failed to load config file: {path}") from e

    def save(self, path: Path | None = None) -> "Config":
        if not path:
            path = DEFAULT_CONFIG_PATH
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("w") as f:
                f.write(Config.model_dump_json(self, indent=4))
            return self
        except Exception as e:
            raise RuntimeError(f"Failed to save config file: {path}") from e


def init_settings() -> Config:
    if not DEFAULT_CONFIG_PATH.exists():
        # 配置文件不存在的情况下，写入空的默认值
        Config().save(DEFAULT_CONFIG_PATH)
    # 读取配置文件，校验出错会抛出异常，校验通过则重新保存一下配置文件（写入新配置项的默认值）
    return Config.load(DEFAULT_CONFIG_PATH).save()


settings = init_settings()
