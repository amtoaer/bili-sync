import os
from asyncio import create_subprocess_exec
from pathlib import Path

from tortoise import Tortoise, fields
from tortoise.fields import Field
from tortoise.models import Model

from constants import DEFAULT_THUMB_PATH, MIGRATE_COMMAND, TORTOISE_ORM, MediaStatus, MediaType
from settings import settings
from version import VERSION


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
        return DEFAULT_THUMB_PATH / str(self.mid)[0] / f"{self.mid}" / "folder.jpg"

    @property
    def meta_path(self) -> Path:
        return DEFAULT_THUMB_PATH / str(self.mid)[0] / f"{self.mid}" / "person.nfo"


class FavoriteItem(Model):
    """收藏条目"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    type = fields.IntEnumField(enum_type=MediaType)
    status = fields.IntEnumField(enum_type=MediaStatus, default=MediaStatus.NORMAL)
    bvid = fields.CharField(max_length=255)
    desc = fields.TextField()
    cover = fields.TextField()
    tags = fields.JSONField(null=True)
    favorite_list: Field[FavoriteList] = fields.ForeignKeyField("models.FavoriteList", related_name="items")
    upper: Field[Upper] = fields.ForeignKeyField("models.Upper", related_name="uploads")
    ctime = fields.DatetimeField()
    pubtime = fields.DatetimeField()
    fav_time = fields.DatetimeField()
    downloaded = fields.BooleanField(default=False)
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)

    class Meta:
        unique_together = (("bvid", "favorite_list_id"),)

    @property
    def safe_name(self) -> str:
        return self.name.replace("/", "_")

    @property
    def tmp_video_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"tmp_{self.bvid}_video"

    @property
    def tmp_audio_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"tmp_{self.bvid}_audio"

    @property
    def video_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"{self.bvid}.mp4"

    @property
    def nfo_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"{self.bvid}.nfo"

    @property
    def poster_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"{self.bvid}-poster.jpg"

    @property
    def upper_path(self) -> list[Path]:
        return [self.upper.thumb_path, self.upper.meta_path]

    @property
    def subtitle_path(self) -> Path:
        return Path(settings.path_mapper[self.favorite_list_id]) / f"{self.bvid}.zh-CN.default.ass"

    @property
    def tvshow_nfo_path(self) -> Path:
        """分p视频时使用"""
        return Path(settings.path_mapper[self.favorite_list_id]) / self.bvid / "tvshow.nfo"

    @property
    def tvshow_poster_path(self) -> Path:
        """分p视频时使用"""
        return Path(settings.path_mapper[self.favorite_list_id]) / self.bvid / "poster.jpg"


class FavoriteItemPage(Model):
    """收藏条目的分p"""

    id = fields.IntField(pk=True)
    favorite_item: Field[FavoriteItem] = fields.ForeignKeyField("models.FavoriteItem", related_name="pages")
    cid = fields.IntField()
    page = fields.IntField()
    name = fields.CharField(max_length=255)
    image = fields.TextField()
    status = fields.IntEnumField(enum_type=MediaStatus, default=MediaStatus.NORMAL)
    downloaded = fields.BooleanField(default=False)

    class Meta:
        unique_together = (("favorite_item_id", "page"),)

    @property
    def tmp_video_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"tmp_{self.favorite_item.bvid} - S01E{':02d'.format()}_video"
        )

    @property
    def tmp_audio_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"tmp_{self.favorite_item.bvid} - S01E{':02d'.format()}_audio"
        )

    @property
    def video_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"{self.favorite_item.bvid} - S01E{':02d'.format()}.mp4"
        )

    @property
    def nfo_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"{self.favorite_item.bvid} - S01E{':02d'.format()}.nfo"
        )

    @property
    def poster_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"{self.favorite_item.bvid} - S01E{':02d'.format()}-thumb.jpg"
        )

    @property
    def subtitle_path(self) -> Path:
        return (
            Path(settings.path_mapper[self.favorite_list_id])
            / self.favorite_item.bvid
            / "Season 1"
            / f"{self.favorite_item.bvid} - S01E{':02d'.format()}.zh-CN.default.ass"
        )


class Program(Model):
    id = fields.IntField(pk=True)
    version = fields.CharField(max_length=20)


async def init_model() -> None:
    await Tortoise.init(config=TORTOISE_ORM)
    migrate_commands = (
        [MIGRATE_COMMAND, "upgrade"] if os.getenv("BILI_IN_DOCKER") else ["poetry", "run", MIGRATE_COMMAND, "upgrade"]
    )
    process = await create_subprocess_exec(*migrate_commands)
    await process.communicate()
    program, created = await Program.get_or_create(defaults={"version": VERSION})
    if created or program.version != VERSION:
        # 把新版本的迁移逻辑写在这里
        pass
    program.version = VERSION
    await program.save()
