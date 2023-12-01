import asyncio

from loguru import logger

from constants import MediaStatus, MediaType
from models import FavoriteItem, Upper
from processor import download_content
from utils import aexists, amakedirs


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


async def upper_thumb():
    makedir_tasks = []
    other_tasks = []
    for upper in await Upper.all():
        if all(
            await asyncio.gather(
                aexists(upper.thumb_path), aexists(upper.meta_path)
            )
        ):
            logger.info(
                "Upper {} {} already exists, skipped.", upper.mid, upper.name
            )
        makedir_tasks.append(amakedirs(upper.thumb_path.parent, exist_ok=True))
        logger.info("Saving metadata for upper {} {}...", upper.mid, upper.name)
        other_tasks.extend(
            [
                upper.save_metadata(),
                download_content(upper.thumb, upper.thumb_path),
            ]
        )
    await asyncio.gather(*makedir_tasks)
    await asyncio.gather(*other_tasks)
    logger.info("All done.")
