from asyncio import create_subprocess_exec
from pathlib import Path

from tortoise import Tortoise, fields
from tortoise.models import Model

from constants import (
    DEFAULT_THUMB_PATH,
    MIGRATE_COMMAND,
    TORTOISE_ORM,
    MediaType,
)
from settings import settings


class FavoriteList(Model):
    """收藏列表"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)

    @property
    def video_list_path(self) -> Path:
        return Path(settings.path_mapper[self.id])


class Upper(Model):
    """up主"""

    mid = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    thumb = fields.TextField()
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)

    @property
    def thumb_path(self) -> Path:
        return DEFAULT_THUMB_PATH / f"{self.mid}.jpg"


class FavoriteItem(Model):
    """收藏条目"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    type = fields.IntEnumField(enum_type=MediaType)
    bvid = fields.CharField(max_length=255)
    desc = fields.TextField()
    cover = fields.TextField()
    favorite_list = fields.ForeignKeyField(
        "models.FavoriteList", related_name="items"
    )
    upper = fields.ForeignKeyField("models.Upper", related_name="uploads")
    ctime = fields.DatetimeField()
    pubtime = fields.DatetimeField()
    fav_time = fields.DatetimeField()
    downloaded = fields.BooleanField(default=False)

    class Meta:
        unique_together = (("bvid", "favorite_list_id"),)

    @property
    def safe_name(self) -> str:
        return self.name.replace("/", "_")

    @property
    def tmp_video_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / f"tmp_{self.bvid}_video"
        )

    @property
    def tmp_audio_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / f"tmp_{self.bvid}_audio"
        )

    @property
    def video_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / f"{self.bvid}.mp4"
        )

    @property
    def nfo_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / f"{self.bvid}.nfo"
        )

    @property
    def poster_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / f"{self.bvid}-poster.jpg"
        )


async def init_model() -> None:
    await Tortoise.init(config=TORTOISE_ORM)
    process = await create_subprocess_exec(
        "poetry",
        "run",
        MIGRATE_COMMAND,
        "upgrade",
    )
    await process.communicate()
