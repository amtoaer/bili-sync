import asyncio
import sys

from processor import process
from settings import settings


async def entry() -> None:
    if any("once" in _ for _ in sys.argv):
        # 单次运行
        await process()
    while True:
        await process()
        await asyncio.sleep(settings.interval * 60)


def start() -> None:
    asyncio.run(entry())
