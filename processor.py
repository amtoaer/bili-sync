import asyncio
import datetime
from asyncio import Semaphore, create_subprocess_exec
from asyncio.subprocess import DEVNULL
from pathlib import Path

import aiofiles
import httpx
from bilibili_api import HEADERS, favorite_list, video
from loguru import logger

from constants import FFMPEG_COMMAND, MediaType
from credential import credential
from nfo import Actor, EpisodeInfo
from settings import settings

anchor = datetime.datetime.today()

client = httpx.AsyncClient(headers=HEADERS)


async def cleanup() -> None:
    await client.aclose()


def concurrent_decorator(concurrency: int) -> callable:
    sem = Semaphore(value=concurrency)

    def decorator(func: callable) -> callable:
        async def wrapper(*args, **kwargs) -> any:
            async with sem:
                return await func(*args, **kwargs)

        return wrapper

    return decorator


async def download_content(url: str, path: Path):
    async with client.stream("GET", url) as resp, aiofiles.open(
        path, "wb"
    ) as f:
        async for chunk in resp.aiter_bytes(40960):
            if not chunk:
                return
            await f.write(chunk)


async def process():
    global anchor
    if (
        datetime.datetime.now() - anchor
    ).days >= 1 and await credential.check_refresh():
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
    save_path = Path(settings.path_mapper[favorite_id])
    save_path.mkdir(parents=True, exist_ok=True)
    page, tasks = 0, []
    while True:
        page += 1
        favorite_video_list = (
            await favorite_list.get_video_favorite_list_content(
                favorite_id, page=page, credential=credential
            )
        )
        if page == 1:
            logger.info(
                "start to process favorite {}: {}",
                favorite_id,
                favorite_video_list["info"]["title"],
            )
        tasks.extend(
            [
                process_video(save_path, media)
                for media in favorite_video_list["medias"]
            ]
        )
        if not favorite_video_list["has_more"]:
            break
    await asyncio.gather(*tasks, return_exceptions=True)


@concurrent_decorator(4)
async def process_video(save_path: Path, media: dict) -> None:
    title = media["title"]
    safe_title = media["title"].replace("/", "_")
    logger.info("start to process video {}", title)
    if media["type"] != MediaType.VIDEO:
        logger.warning("Media {} is not a video, skipped.", title)
        return
    final_path = save_path / f"{safe_title}.mp4"
    if final_path.exists():
        logger.info(f"{final_path} already exists, skipped.")
        return
    # 写入 nfo
    nfo_path = save_path / f"{safe_title}.nfo"
    EpisodeInfo(
        title=title,
        plot=media["intro"],
        actor=[Actor(f"{media['upper']['mid']} - {media['upper']['name']}")],
        bvid=media["bvid"],
        aired=datetime.datetime.fromtimestamp(media["ctime"]),
    ).write_nfo(nfo_path)
    # 写入 poster
    cover_path = save_path / f"{safe_title}-poster.jpg"
    await download_content(media["cover"], cover_path)
    # 开始处理视频内容
    v = video.Video(media["bvid"], credential=credential)
    detector = video.VideoDownloadURLDataDetecter(
        await v.get_download_url(page_index=0)
    )
    streams = detector.detect_best_streams()
    if detector.check_flv_stream():
        tmp_path = save_path / f"{safe_title}.flv"
        await download_content(streams[0].url, tmp_path)
        process = await create_subprocess_exec(
            FFMPEG_COMMAND,
            "-i",
            str(tmp_path),
            str(final_path),
            stdout=DEVNULL,
            stderr=DEVNULL,
        )
        await process.communicate()
        tmp_path.unlink()
    else:
        tmp_video_path, tmp_audio_path = (
            save_path / f"{safe_title}_video.m4s",
            save_path / f"{safe_title}_audio.m4s",
        )
        await asyncio.gather(
            download_content(streams[0].url, tmp_video_path),
            download_content(streams[1].url, tmp_audio_path),
        )
        process = await create_subprocess_exec(
            FFMPEG_COMMAND,
            "-i",
            str(tmp_video_path),
            "-i",
            str(tmp_audio_path),
            "-c",
            "copy",
            str(final_path),
            stdout=DEVNULL,
            stderr=DEVNULL,
        )
        await process.communicate()
        tmp_video_path.unlink()
        tmp_audio_path.unlink()
    logger.info(f"{title} downloaded successfully.")
