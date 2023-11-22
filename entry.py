import asyncio
import sys

from loguru import logger

from processor import process
from settings import settings


async def entry() -> None:
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
    asyncio.run(entry())
