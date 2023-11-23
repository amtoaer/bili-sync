import os
from enum import IntEnum
from pathlib import Path

DEFAULT_CONFIG_PATH = (
    Path(__file__).parent / "config.json"
    if not os.getenv("TESTING")
    else Path(__file__).parent / "config.test.json"
)

DEFAULT_DATABASE_PATH = (
    Path(__file__).parent / "database.db"
    if not os.getenv("TESTING")
    else Path(__file__).parent / "database.test.db"
)

FFMPEG_COMMAND = "ffmpeg"


class MediaType(IntEnum):
    VIDEO = 2
    AUDIO = 12
    VIDEO_COLLECTION = 21
