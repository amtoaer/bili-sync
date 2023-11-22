from pathlib import Path
import os

DEFAULT_CONFIG_PATH = (
    Path(__file__).parent / "config.json"
    if not os.getenv("TESTING")
    else Path(__file__).parent / "config.test.json"
)

FFMPEG_COMMAND = "ffmpeg"
