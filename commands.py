import asyncio
import functools
from pathlib import Path
from typing import Callable

from loguru import logger

from constants import MediaStatus, MediaType
from models import FavoriteItem
from processor import process_favorite_item
from utils import aexists, aremove


async def recheck():
    """刷新数据库中视频的状态，如果发现文件不存在则标记未下载，以便在下次任务重新下载，在自己手动删除文件后调用"""
    items = await FavoriteItem.filter(
        type=MediaType.VIDEO,
        status=MediaStatus.NORMAL,
        downloaded=True,
    )
    exists = await asyncio.gather(*[aexists(item.video_path) for item in items])
    for item, exist in zip(items, exists):
        if isinstance(exist, Exception):
            logger.error(
                "Error when checking file {} {}: {}",
                item.bvid,
                item.name,
                exist,
            )
            continue
        if not exist:
            logger.info(
                "File {} {} not exists, mark as not downloaded.",
                item.bvid,
                item.name,
            )
            item.downloaded = False
    logger.info("Updating database...")
    await FavoriteItem.bulk_update(items, fields=["downloaded"])
    logger.info("Database updated.")


async def _refresh_favorite_item_info(
    path_getter: Callable[[FavoriteItem], list[Path]],
    process_poster: bool = False,
    process_video: bool = False,
    process_nfo: bool = False,
    process_upper: bool = False,
    process_subtitle: bool = False,
    force: bool = False,
):
    items = await FavoriteItem.filter(downloaded=True).prefetch_related("upper")
    if force:
        # 如果强制刷新，那么就先把现存的所有内容删除
        await asyncio.gather(
            *[aremove(path) for item in items for path in path_getter(item)],
            return_exceptions=True,
        )
    await asyncio.gather(
        *[
            process_favorite_item(
                item,
                process_poster=process_poster,
                process_video=process_video,
                process_nfo=process_nfo,
                process_upper=process_upper,
                process_subtitle=process_subtitle,
            )
            for item in items
        ],
        return_exceptions=True,
    )


refresh_nfo = functools.partial(
    _refresh_favorite_item_info, lambda item: [item.nfo_path], process_nfo=True
)

refresh_poster = functools.partial(
    _refresh_favorite_item_info,
    lambda item: [item.poster_path],
    process_poster=True,
)

refresh_video = functools.partial(
    _refresh_favorite_item_info,
    lambda item: [item.video_path],
    process_video=True,
)

refresh_upper = functools.partial(
    _refresh_favorite_item_info,
    lambda item: item.upper_path,
    process_upper=True,
)

refresh_subtitle = functools.partial(
    _refresh_favorite_item_info,
    lambda item: [item.subtitle_path],
    process_subtitle=True,
)
