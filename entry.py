import asyncio
import sys

import uvloop
from loguru import logger

from commands import (
    recheck,
    refresh_nfo,
    refresh_poster,
    refresh_subtitle,
    refresh_upper,
    refresh_video,
)
from models import init_model
from processor import cleanup, process
from settings import settings

asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())


async def entry() -> None:
    await init_model()
    for command, func in (
        ("once", process),
        ("recheck", recheck),
        ("refresh_poster", refresh_poster),
        ("refresh_upper", refresh_upper),
        ("refresh_nfo", refresh_nfo),
        ("refresh_video", refresh_video),
        ("refresh_subtitle", refresh_subtitle),
    ):
        if any(command in _ for _ in sys.argv):
            logger.info("Running {}...", command)
            await func()
            return
    logger.info("Running daemon...")
    while True:
        await process()
        await asyncio.sleep(settings.interval * 60)


if __name__ == "__main__":
    with asyncio.Runner() as runner:
        try:
            runner.run(entry())
        finally:
            runner.run(cleanup())
