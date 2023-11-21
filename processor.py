from constants import FFMPEG_COMMAND
from settings import settings
from credential import credential
from bilibili_api import favorite_list, video, HEADERS
from pathlib import Path
import httpx
from asyncio import create_subprocess_exec
from asyncio.subprocess import DEVNULL
from loguru import logger


async def download_content(url: str, path: Path):
    async with httpx.AsyncClient(headers=HEADERS) as sess:
        resp = await sess.get(url)
        with path.open("wb") as f:
            for chunk in resp.iter_bytes(1024):
                if not chunk:
                    break
                f.write(chunk)


async def process():
    for favorite_id in settings.favorite_ids:
        if favorite_id not in settings.path_mapper:
            logger.warning(f"Favorite {favorite_id} not in path mapper, ignored.")
            continue
        try:
            save_path = Path(settings.path_mapper[favorite_id])
            save_path.mkdir(parents=True, exist_ok=True)
            favorite_video_list = await favorite_list.get_video_favorite_list_content(
                favorite_id, credential=credential
            )
            logger.info(
                "start to process favorite {}", favorite_video_list["info"]["title"]
            )
            # TODO:
            # 1. 添加进度条
            # 2. 翻页以下载全部内容
            # 3. 对接数据库
            # 4. 构建 nfo
            for media in favorite_video_list["medias"][:8]:
                title = media["title"]
                final_path = save_path / f"{title}.mp4"
                if final_path.exists():
                    logger.info(f"{final_path} already exists, skipped.")
                    continue
                v = video.Video(media["bvid"], credential=credential)
                detector = video.VideoDownloadURLDataDetecter(
                    await v.get_download_url(page_index=0)
                )
                streams = detector.detect_best_streams()
                if detector.check_flv_stream():
                    tmp_path = save_path / f"{title}.flv"
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
                        save_path / f"{title}_video.m4s",
                        save_path / f"{title}_audio.m4s",
                    )
                    await download_content(streams[0].url, tmp_video_path)
                    await download_content(streams[1].url, tmp_audio_path)
                    # 混流
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

                logger.info(f"{final_path} downloaded successfully.")

        except Exception:
            logger.exception(f"Failed to process favorite {favorite_id}")
            continue
