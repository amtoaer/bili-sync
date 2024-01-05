import asyncio
import os
import signal
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
    force = any("force" in _ for _ in sys.argv)
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
            if command.startswith("refresh"):
                await func(force=force)
            else:
                await func()
            return
    logger.info("Running daemon...")
    while True:
        await process()
        await asyncio.sleep(settings.interval * 60)


if __name__ == "__main__":
    # 确保 docker 退出时正确触发资源释放
    signal.signal(
        signal.SIGTERM, lambda *_: os.kill(os.getpid(), signal.SIGINT)
    )
    with asyncio.Runner() as runner:
        try:
            runner.run(entry())
        except Exception:
            logger.exception("Unexpected error occurred, exiting...")
        except KeyboardInterrupt:
            logger.error("Exit Signal Received, exiting...")
        finally:
            logger.info("Cleaning up resources...")
            runner.run(cleanup())
            logger.info("Done, exited.")
