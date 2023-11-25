import os
from enum import IntEnum
from pathlib import Path


def get_base(dir_name: str) -> Path:
    path = (
        Path(base)
        if (base := os.getenv(f"{dir_name.upper()}_PATH"))
        else Path(__file__).parent / dir_name
    )
    path.mkdir(parents=True, exist_ok=True)
    return path


DEFAULT_CONFIG_PATH = get_base("config") / "config.json"

DEFAULT_DATABASE_PATH = get_base("data") / "data.db"

DEFAULT_THUMB_PATH = get_base("thumb")

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
