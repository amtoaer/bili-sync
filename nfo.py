import datetime
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Actor:
    name: str

    def to_xml(self) -> str:
        return f"""
    <actor>
        <name>{self.name}</name>
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

    def write_nfo(self, path: Path) -> None:
        with path.open("w", encoding="utf-8") as f:
            f.write(self.to_xml())

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
