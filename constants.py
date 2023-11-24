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

DEFAULT_THUMB_PATH = (
    Path(__file__).parent / "thumbs"
    if not os.getenv("TESTING")
    else Path(__file__).parent / "thumbs.test"
)

FFMPEG_COMMAND = "ffmpeg"

MIGRATE_COMMAND = "aerich"


class MediaType(IntEnum):
    VIDEO = 2
    AUDIO = 12
    VIDEO_COLLECTION = 21


TORTOISE_ORM = {
    "connections": {"default": f"sqlite://{DEFAULT_DATABASE_PATH}"},
    "apps": {
        "models": {
            "models": ["models", "aerich.models"],
            "default_connection": "default",
        },
    },
    "use_tz": True,
}
