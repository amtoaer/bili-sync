from pathlib import Path

from bilibili_api.video import AudioQuality, VideoCodecs, VideoQuality
from pydantic import BaseModel, Field, field_validator, root_validator
from pydantic_core import PydanticCustomError
from typing_extensions import Annotated

from constants import DEFAULT_CONFIG_PATH
from utils import amakedirs, aopen


class SubtitleConfig(BaseModel):
    font_name: str = "微软雅黑，黑体"  # 字体
    font_size: float = 40  # 字号
    alpha: float = 0.8  # 透明度
    fly_time: float = 5  # 滚动弹幕持续时间
    static_time: float = 10  # 静态弹幕持续时间


class StreamConfig(BaseModel):
    video_max_quality: VideoQuality = VideoQuality._8K
    audio_max_quality: AudioQuality = AudioQuality._192K
    video_min_quality: VideoQuality = VideoQuality._360P
    audio_min_quality: AudioQuality = AudioQuality._64K
    codecs: list[VideoCodecs] = Field(
        default_factory=lambda: [VideoCodecs.AV1, VideoCodecs.AVC, VideoCodecs.HEV], min_length=1
    )
    no_dolby_video: bool = False
    no_dolby_audio: bool = False
    no_hdr: bool = False
    no_hires: bool = False

    @field_validator("codecs", mode="after")
    def codec_validator(cls, codecs: list[VideoCodecs]) -> list[VideoCodecs]:
        if len(codecs) != len(set(codecs)):
            raise PydanticCustomError("unique_list", "List must be unique")
        return codecs


class Config(BaseModel):
    sessdata: Annotated[str, Field(min_length=1)] = ""
    bili_jct: Annotated[str, Field(min_length=1)] = ""
    buvid3: Annotated[str, Field(min_length=1)] = ""
    dedeuserid: Annotated[str, Field(min_length=1)] = ""
    ac_time_value: Annotated[str, Field(min_length=1)] = ""
    interval: int = 20
    path_mapper: dict[int, str] = Field(default_factory=dict)
    subtitle: SubtitleConfig = Field(default_factory=SubtitleConfig)
    stream: StreamConfig = Field(default_factory=StreamConfig)
    paginated_video: bool = False

    @root_validator(pre=True)
    def migrate(cls, values: dict) -> dict:
        # 把旧版本的 codec 迁移为 stream 中的 codecs
        if "codec" in values and "stream" not in values:
            values["stream"] = {"codecs": values.pop("codec")}
        return values

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

    async def asave(self, path: Path | None = None) -> "Config":
        if not path:
            path = DEFAULT_CONFIG_PATH
        try:
            await amakedirs(path.parent, exist_ok=True)
            async with aopen(path, "w") as f:
                await f.write(Config.model_dump_json(self, indent=4))
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
