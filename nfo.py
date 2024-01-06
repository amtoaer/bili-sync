import datetime
from dataclasses import dataclass
from pathlib import Path

from utils import aopen


@dataclass
class EpositeInfo:
    """分p的单集信息"""

    title: str
    season: int
    episode: int

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
class Actor:
    """在视频信息中嵌入的up主信息"""

    name: str
    role: str

    def to_xml(self) -> str:
        return f"""
    <actor>
        <name>{self.name}</name>
        <role>{self.role}</role>
    </actor>
""".strip()


@dataclass
class MovieInfo:
    """单p的视频信息"""

    title: str
    plot: str
    tags: list[str]
    actor: list[Actor]
    bvid: str
    aired: datetime.datetime

    async def write_nfo(self, path: Path) -> None:
        async with aopen(path, "w", encoding="utf-8") as f:
            await f.write(self.to_xml())

    def to_xml(self) -> str:
        actor = "\n".join(_.to_xml() for _ in self.actor)
        tags = (
            "\n".join(f"    <genre>{_}</genre>" for _ in self.tags)
            if isinstance(self.tags, list)
            else ""
        )
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
class TVShowInfo:
    """分p的总集信息，和 movie 除标签外保持一致"""

    title: str
    plot: str
    tags: list[str]
    actor: list[Actor]
    bvid: str
    aired: datetime.datetime

    async def write_nfo(self, path: Path) -> None:
        async with aopen(path, "w", encoding="utf-8") as f:
            await f.write(self.to_xml())

    def to_xml(self) -> str:
        actor = "\n".join(_.to_xml() for _ in self.actor)
        tags = (
            "\n".join(f"    <genre>{_}</genre>" for _ in self.tags)
            if isinstance(self.tags, list)
            else ""
        )
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
class UpperInfo:
    mid: int
    created_at: datetime.datetime

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
