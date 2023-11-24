import asyncio
import sys

import uvloop
from loguru import logger

from models import init_model
from processor import cleanup, process
from settings import settings

asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())


async def entry() -> None:
    await init_model()
    if any("once" in _ for _ in sys.argv):
        # 单次运行
        logger.info("Running once...")
        await process()
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
