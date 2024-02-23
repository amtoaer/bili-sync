import datetime
from abc import abstractmethod
from dataclasses import dataclass
from pathlib import Path

from models import FavoriteItem, FavoriteItemPage, Upper
from utils import aopen


@dataclass
class Base:
    """基类，有个工具方法"""

    @abstractmethod
    def to_xml(self) -> str:
        ...

    async def to_file(self, path: Path) -> None:
        """把 xml 写入文件"""
        async with aopen(path, "w", encoding="utf-8") as f:
            await f.write(self.to_xml())


@dataclass
class EpisodeInfo(Base):
    """分p的单集信息"""

    title: str
    season: int
    episode: int

    @staticmethod
    def from_favorite_item_page(page: FavoriteItemPage) -> "EpisodeInfo":
        return EpisodeInfo(title=page.title, season=page.season, episode=page.episode)

    def to_xml(self) -> str:
        return f"""
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<episodedetails>
    <plot />
    <outline />
    <title>{self.title}</title>
    <season>{self.season}</season>
    <episode>{self.episode}</episode>
</episodedetails>
""".strip()


@dataclass
class Actor(Base):
    name: str
    role: str

    @staticmethod
    def from_upper(upper: Upper) -> "Actor":
        return Actor(name=upper.mid, role=upper.name)

    def to_xml(self) -> str:
        return f"""
    <actor>
        <name>{self.name}</name>
        <role>{self.role}</role>
    </actor>
""".strip()


@dataclass
class MovieInfo(Base):
    """单p的视频信息"""

    title: str
    plot: str
    tags: list[str]
    actor: list[Actor]
    bvid: str
    aired: datetime.datetime

    @staticmethod
    def from_favorite_item(fav_item: FavoriteItem) -> "MovieInfo":
        return MovieInfo(
            title=fav_item.name,
            plot=fav_item.desc,
            actor=[Actor.from_upper(upper) for upper in fav_item.upper],
            tags=fav_item.tags,
            bvid=fav_item.bvid,
            aired=fav_item.ctime,
        )

    def to_xml(self) -> str:
        actor = "\n".join(_.to_xml() for _ in self.actor)
        tags = "\n".join(f"    <genre>{_}</genre>" for _ in self.tags) if isinstance(self.tags, list) else ""
        return f"""
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<movie>
    <plot><![CDATA[{self.plot}]]></plot>
    <outline />
    <title>{self.title}</title>
{actor}
    <year>{self.aired.year}</year>
{tags}
    <uniqueid type="bilibili">{self.bvid}</uniqueid>
    <aired>{self.aired.strftime("%Y-%m-%d")}</aired>
</movie>
""".strip()


@dataclass
class TVShowInfo(Base):
    title: str
    plot: str
    tags: list[str]
    actor: list[Actor]
    bvid: str
    aired: datetime.datetime

    @staticmethod
    def from_favorite_item(fav_item: FavoriteItem) -> "TVShowInfo":
        return TVShowInfo(
            title=fav_item.name,
            plot=fav_item.desc,
            actor=[Actor.from_upper(upper) for upper in fav_item.upper],
            tags=fav_item.tags,
            bvid=fav_item.bvid,
            aired=fav_item.ctime,
        )

    def to_xml(self) -> str:
        actor = "\n".join(_.to_xml() for _ in self.actor)
        tags = "\n".join(f"    <genre>{_}</genre>" for _ in self.tags) if isinstance(self.tags, list) else ""
        return f"""
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<tvshow>
    <plot><![CDATA[{self.plot}]]></plot>
    <outline />
    <title>{self.title}</title>
{actor}
    <year>{self.aired.year}</year>
{tags}
    <uniqueid type="bilibili">{self.bvid}</uniqueid>
    <aired>{self.aired.strftime("%Y-%m-%d")}</aired>
</tvshow>
""".strip()


@dataclass
class UpperInfo(Base):
    mid: int
    created_at: datetime.datetime

    def from_upper(upper: Upper) -> "UpperInfo":
        return UpperInfo(mid=upper.mid, created_at=upper.created_at)

    def to_xml(self) -> str:
        return f"""
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<person>
    <plot />
    <outline />
    <lockdata>false</lockdata>
    <dateadded>{self.created_at.strftime("%Y-%m-%d %H:%M:%S")}</dateadded>
    <title>{self.mid}</title>
    <sorttitle>{self.mid}</sorttitle>
</person>
""".strip()
