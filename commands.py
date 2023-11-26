import asyncio

from aiofiles.os import path
from loguru import logger

from constants import MediaStatus, MediaType
from models import FavoriteItem


async def recheck():
    """刷新数据库中视频的状态，如果发现文件不存在则标记未下载，以便在下次任务重新下载，在自己手动删除文件后调用"""
    items = await FavoriteItem.filter(
        type=MediaType.VIDEO,
        status=MediaStatus.NORMAL,
        downloaded=True,
    )
    exists = await asyncio.gather(
        *[path.exists(item.video_path) for item in items]
    )
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
