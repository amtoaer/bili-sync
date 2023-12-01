import datetime
from dataclasses import dataclass
from pathlib import Path

from utils import aopen


@dataclass
class Actor:
    name: str
    role: str

    def to_xml(self) -> str:
        return f"""
    <actor>
        <name>{self.name}</name>
        <role>{self.role}</role>
    </actor>
""".strip(
            "\n"
        )


@dataclass
class EpisodeInfo:
    title: str
    plot: str
    actor: list[Actor]
    bvid: str
    aired: datetime.datetime

    async def write_nfo(self, path: Path) -> None:
        async with aopen(path, "w", encoding="utf-8") as f:
            await f.write(self.to_xml())

    def to_xml(self) -> str:
        actor = "\n".join(_.to_xml() for _ in self.actor)
        return f"""
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<episodedetails>
    <plot><![CDATA[{self.plot}]]></plot>
    <outline />
    <title>{self.title}</title>
{actor}
    <year>{self.aired.year}</year>
    <uniqueid type="bilibili">{self.bvid}</uniqueid>
    <aired>{self.aired.strftime("%Y-%m-%d")}</aired>
</episodedetails>
""".strip(
            "\n"
        )
