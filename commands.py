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

    async def is_ok(item: FavoriteItem) -> bool:
        if len(item.pages):
            # 多 p 视频全部存在才算存在
            return all(await asyncio.gather(*[aexists(page.video_path) for page in item.pages]))
        return await aexists(item.video_path)

    items = await FavoriteItem.filter(
        type=MediaType.VIDEO, status=MediaStatus.NORMAL, downloaded=True
    ).prefetch_related("pages")
    items_to_update = []
    for item in items:
        for page in item.pages:
            # 疑似 tortoise 的 bug，prefetch_related 不会更新反向引用的字段，这里手动更新一下
            page.favorite_item = item
    items_ok = await asyncio.gather(*[is_ok(item) for item in items], return_exceptions=True)
    for item, ok in zip(items, items_ok):
        if isinstance(ok, Exception):
            logger.error("Error when checking file {} {}: {}.", item.bvid, item.name, ok)
            continue
        if not ok:
            logger.info("Lack of file detected for {} {}, mark as not downloaded.", item.bvid, item.name)
            item.downloaded = False
            items_to_update.append(item)
    logger.info("Updating database...")
    await FavoriteItem.bulk_update(items_to_update, fields=["downloaded"], batch_size=300)
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
        await asyncio.gather(*[aremove(path) for item in items for path in path_getter(item)], return_exceptions=True)
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


refresh_nfo = functools.partial(_refresh_favorite_item_info, lambda item: [item.nfo_path], process_nfo=True)

refresh_poster = functools.partial(_refresh_favorite_item_info, lambda item: [item.poster_path], process_poster=True)

refresh_video = functools.partial(_refresh_favorite_item_info, lambda item: [item.video_path], process_video=True)

refresh_upper = functools.partial(_refresh_favorite_item_info, lambda item: item.upper_path, process_upper=True)

refresh_subtitle = functools.partial(
    _refresh_favorite_item_info, lambda item: [item.subtitle_path], process_subtitle=True
)
