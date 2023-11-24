import asyncio
import datetime
from asyncio import Semaphore, create_subprocess_exec
from asyncio.subprocess import DEVNULL
from pathlib import Path

import aiofiles
import httpx
from bilibili_api import HEADERS, favorite_list, video
from loguru import logger
from tortoise import Tortoise

from constants import DEFAULT_THUMB_PATH, FFMPEG_COMMAND, MediaType
from credential import credential
from models import FavoriteItem, FavoriteList, Upper
from nfo import Actor, EpisodeInfo
from settings import settings

anchor = datetime.datetime.today()

client = httpx.AsyncClient(headers=HEADERS)


async def cleanup() -> None:
    await client.aclose()
    await Tortoise.close_connections()


def concurrent_decorator(concurrency: int) -> callable:
    sem = Semaphore(value=concurrency)

    def decorator(func: callable) -> callable:
        async def wrapper(*args, **kwargs) -> any:
            async with sem:
                return await func(*args, **kwargs)

        return wrapper

    return decorator


async def download_content(url: str, path: Path) -> None:
    async with client.stream("GET", url) as resp, aiofiles.open(
        path, "wb"
    ) as f:
        async for chunk in resp.aiter_bytes(40960):
            if not chunk:
                return
            await f.write(chunk)


async def manage_model(medias: list[dict], fav_list: FavoriteList) -> None:
    uppers = [
        Upper(
            mid=media["upper"]["mid"],
            name=media["upper"]["name"],
            thumb=media["upper"]["face"],
        )
        for media in medias
    ]
    await Upper.bulk_create(
        uppers, on_conflict=["mid"], update_fields=["name", "thumb"]
    )
    items = [
        FavoriteItem(
            name=media["title"],
            type=media["type"],
            bvid=media["bvid"],
            desc=media["intro"],
            cover=media["cover"],
            favorite_list=fav_list,
            upper_id=media["upper"]["mid"],
            ctime=datetime.datetime.utcfromtimestamp(media["ctime"]),
            pubtime=datetime.datetime.utcfromtimestamp(media["pubtime"]),
            fav_time=datetime.datetime.utcfromtimestamp(media["fav_time"]),
            downloaded=False,
        )
        for media in medias
    ]
    await FavoriteItem.bulk_create(
        items,
        on_conflict=["bvid", "favorite_list_id"],
        update_fields=[
            "name",
            "type",
            "desc",
            "cover",
            "ctime",
            "pubtime",
            "fav_time",
        ],
    )


async def process() -> None:
    global anchor
    if (datetime.datetime.now() - anchor).days >= 3:
        # 暂定三天刷新一次凭据，具体看情况调整
        try:
            credential.refresh()
            anchor = datetime.datetime.today()
            logger.info("Credential refreshed.")
        except Exception:
            logger.exception("Failed to refresh credential.")
            return
    for favorite_id in settings.favorite_ids:
        if favorite_id not in settings.path_mapper:
            logger.warning(
                f"Favorite {favorite_id} not in path mapper, ignored."
            )
            continue
        await process_favorite(favorite_id)


async def process_favorite(favorite_id: int) -> None:
    # 预先请求第一页内容以获取收藏夹标题
    favorite_video_list = await favorite_list.get_video_favorite_list_content(
        favorite_id, page=1, credential=credential
    )
    logger.info(
        "start to process favorite {}: {}",
        favorite_id,
        favorite_video_list["info"]["title"],
    )
    fav_list, _ = await FavoriteList.get_or_create(
        id=favorite_id, defaults={"name": favorite_video_list["info"]["title"]}
    )
    fav_list.video_list_path.mkdir(parents=True, exist_ok=True)
    DEFAULT_THUMB_PATH.mkdir(parents=True, exist_ok=True)
    page = 0
    while True:
        page += 1
        if page > 1:
            favorite_video_list = (
                await favorite_list.get_video_favorite_list_content(
                    favorite_id, page=page, credential=credential
                )
            )
        # 先看看对应 bvid 的记录是否存在
        existed_items = await FavoriteItem.filter(
            favorite_list=fav_list,
            bvid__in=[media["bvid"] for media in favorite_video_list["medias"]],
        )
        # 记录一下获得的列表中的 bvid 和 fav_time
        media_info = {
            (media["bvid"], media["fav_time"])
            for media in favorite_video_list["medias"]
        }
        # 如果有 bvid 和 fav_time 都相同的记录，说明已经到达了上次处理到的位置
        continue_flag = not media_info & {
            (item.bvid, int(item.fav_time.timestamp()))
            for item in existed_items
        }
        await manage_model(favorite_video_list["medias"], fav_list)
        if not (continue_flag and favorite_video_list["has_more"]):
            break
    all_unprocessed_items = await FavoriteItem.filter(
        favorite_list=fav_list, downloaded=False
    ).prefetch_related("upper")
    await asyncio.gather(
        *[process_video(item) for item in all_unprocessed_items],
        return_exceptions=True,
    )


@concurrent_decorator(4)
async def process_video(fav_item: FavoriteItem) -> None:
    logger.info("start to process video {}", fav_item.name)
    if fav_item.type != MediaType.VIDEO:
        logger.warning("Media {} is not a video, skipped.", fav_item.name)
        return
    if fav_item.video_path.exists():
        fav_item.downloaded = True
        await fav_item.save()
        logger.info(
            "{} {} already exists, skipped.", fav_item.bvid, fav_item.name
        )
        return
    # 写入 up 主头像
    if not fav_item.upper.thumb_path.exists():
        await download_content(fav_item.upper.thumb, fav_item.upper.thumb_path)
    # 写入 nfo
    EpisodeInfo(
        title=fav_item.name,
        plot=fav_item.desc,
        actor=[
            Actor(
                name=fav_item.upper.mid,
                role=fav_item.upper.name,
                thumb=fav_item.upper.thumb_path,
            )
        ],
        bvid=fav_item.bvid,
        aired=fav_item.ctime,
    ).write_nfo(fav_item.nfo_path)
    # 写入 poster
    await download_content(fav_item.cover, fav_item.poster_path)
    # 开始处理视频内容
    v = video.Video(fav_item.bvid, credential=credential)
    detector = video.VideoDownloadURLDataDetecter(
        await v.get_download_url(page_index=0)
    )
    streams = detector.detect_best_streams()
    if detector.check_flv_stream():
        await download_content(streams[0].url, fav_item.tmp_video_path)
        process = await create_subprocess_exec(
            FFMPEG_COMMAND,
            "-i",
            str(fav_item.tmp_video_path),
            str(fav_item.video_path),
            stdout=DEVNULL,
            stderr=DEVNULL,
        )
        await process.communicate()
        fav_item.tmp_video_path.unlink()
    else:
        await asyncio.gather(
            download_content(streams[0].url, fav_item.tmp_video_path),
            download_content(streams[1].url, fav_item.tmp_audio_path),
        )
        process = await create_subprocess_exec(
            FFMPEG_COMMAND,
            "-i",
            str(fav_item.tmp_video_path),
            "-i",
            str(fav_item.tmp_audio_path),
            "-c",
            "copy",
            str(fav_item.video_path),
            stdout=DEVNULL,
            stderr=DEVNULL,
        )
        await process.communicate()
        fav_item.tmp_video_path.unlink()
        fav_item.tmp_audio_path.unlink()
    fav_item.downloaded = True
    await fav_item.save()
    logger.info("{} {} processed successfully.", fav_item.bvid, fav_item.name)
