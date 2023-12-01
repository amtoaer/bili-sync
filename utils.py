from pathlib import Path

import aiofiles
import httpx
from aiofiles.base import AiofilesContextManager
from aiofiles.os import makedirs
from aiofiles.ospath import exists
from aiofiles.threadpool.text import AsyncTextIOWrapper
from bilibili_api import HEADERS

client = httpx.AsyncClient(headers=HEADERS)


async def download_content(url: str, path: Path) -> None:
    async with client.stream("GET", url) as resp, aopen(path, "wb") as f:
        async for chunk in resp.aiter_bytes(40960):
            if not chunk:
                return
            await f.write(chunk)


async def aexists(path: Path) -> bool:
    return await exists(path)


async def amakedirs(path: Path, exist_ok=False) -> None:
    await makedirs(path, exist_ok=exist_ok)


def aopen(
    path: Path, mode: str = "r", **kwargs
) -> AiofilesContextManager[None, None, AsyncTextIOWrapper]:
    return aiofiles.open(path, mode, **kwargs)
