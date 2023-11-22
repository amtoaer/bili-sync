from pathlib import Path
import os
from enum import IntEnum

DEFAULT_CONFIG_PATH = (
    Path(__file__).parent / "config.json"
    if not os.getenv("TESTING")
    else Path(__file__).parent / "config.test.json"
)

FFMPEG_COMMAND = "ffmpeg"


class MediaType(IntEnum):
    VIDEO = 2
    AUDIO = 12
    VIDEO_COLLECTION = 21
