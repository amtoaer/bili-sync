import asyncio
import contextlib
import datetime
from asyncio import Semaphore, create_subprocess_exec
from asyncio.subprocess import DEVNULL
from pathlib import Path

from bilibili_api import ass, favorite_list, video
from bilibili_api.exceptions import ResponseCodeException
from loguru import logger
from tortoise.connection import connections
from tortoise.models import Model

from constants import FFMPEG_COMMAND, MediaStatus, MediaType, NfoMode
from credential import credential
from models import FavoriteItem, FavoriteItemPage, FavoriteList, Upper
from nfo import Base as NfoBase
from nfo import EpisodeInfo, MovieInfo, TVShowInfo, UpperInfo
from settings import settings
from utils import aexists, aremove, client, download_content

anchor = datetime.date.today()


async def cleanup() -> None:
    await client.aclose()
    await connections.close_all()


def concurrent_decorator(concurrency: int) -> callable:
    """一个简单的并发限制装饰器，被装饰的函数同时仅能运行 concurrency 个"""
    sem = Semaphore(value=concurrency)

    def decorator(func: callable) -> callable:
        async def wrapper(*args, **kwargs) -> any:
            async with sem:
                return await func(*args, **kwargs)

        return wrapper

    return decorator


async def update_favorite_item(medias: list[dict], fav_list: FavoriteList) -> None:
    """根据收藏夹里的视频列表更新数据库记录"""
    uppers = [
        Upper(mid=media["upper"]["mid"], name=media["upper"]["name"], thumb=media["upper"]["face"]) for media in medias
    ]
    await Upper.bulk_create(uppers, on_conflict=["mid"], update_fields=["name", "thumb"])
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
        update_fields=["name", "type", "desc", "cover", "ctime", "pubtime", "fav_time"],
    )


async def process() -> None:
    global anchor
    if (today := datetime.date.today()) > anchor:
        anchor = today
        logger.info("Check credential.")
        if await credential.check_refresh():
            try:
                await credential.refresh()
                logger.info("Credential refreshed.")
            except Exception:
                logger.exception("Failed to refresh credential.")
                return
    for favorite_id in settings.path_mapper:
        await process_favorite(favorite_id)


async def process_favorite(favorite_id: int) -> None:
    # 预先请求第一页内容以获取收藏夹标题
    favorite_video_list = await favorite_list.get_video_favorite_list_content(
        favorite_id, page=1, credential=credential
    )
    title = favorite_video_list["info"]["title"]
    logger.info("Start to process favorite {}: {}", favorite_id, title)
    fav_list, _ = await FavoriteList.get_or_create(
        id=favorite_id, defaults={"name": favorite_video_list["info"]["title"]}
    )
    fav_list.video_list_path.mkdir(parents=True, exist_ok=True)
    page = 0
    while True:
        page += 1
        if page > 1:
            favorite_video_list = await favorite_list.get_video_favorite_list_content(
                favorite_id, page=page, credential=credential
            )
        # 先看看对应 bvid 的记录是否存在
        existed_items = await FavoriteItem.filter(
            favorite_list=fav_list, bvid__in=[media["bvid"] for media in favorite_video_list["medias"]]
        )
        # 记录一下获得的列表中的 bvid 和 fav_time
        media_info = {(media["bvid"], media["fav_time"]) for media in favorite_video_list["medias"]}
        # 如果有 bvid 和 fav_time 都相同的记录，说明已经到达了上次处理到的位置
        continue_flag = not media_info & {(item.bvid, int(item.fav_time.timestamp())) for item in existed_items}
        await update_favorite_item(favorite_video_list["medias"], fav_list)
        if not (continue_flag and favorite_video_list["has_more"]):
            break
    all_unprocessed_items = await FavoriteItem.filter(
        favorite_list=fav_list, type=MediaType.VIDEO, status=MediaStatus.NORMAL, downloaded=False
    ).prefetch_related("upper")
    await asyncio.gather(*[process_favorite_item(item) for item in all_unprocessed_items], return_exceptions=True)
    logger.info("Favorite {} {} processed successfully.", favorite_id, title)


@concurrent_decorator(concurrency=4)
async def process_favorite_item(
    fav_item: FavoriteItem,
    process_poster=True,
    process_video=True,
    process_nfo=True,
    process_upper=True,
    process_subtitle=True,
) -> None:
    logger.info("Start to process video {} {}", fav_item.bvid, fav_item.name)
    if fav_item.type != MediaType.VIDEO:
        logger.warning("Media {} is not a video, skipped.", fav_item.name)
        return
    v = video.Video(fav_item.bvid, credential=credential)
    # 如果没有获取过 tags，那么尝试获取一下（不关键，忽略掉错误）
    with contextlib.suppress(Exception):
        if fav_item.tags is None:
            fav_item.tags = [_["tag_name"] for _ in await v.get_tags()]
    # 处理 up 主信息和是否分 p 无关，放到前面
    if process_upper:
        result = await asyncio.gather(
            get_file(fav_item.upper.thumb, fav_item.upper.thumb_path),
            get_nfo(fav_item.upper.meta_path, obj=fav_item.upper, mode=NfoMode.UPPER),
            return_exceptions=True,
        )
        if any(isinstance(_, FileExistsError) for _ in result):
            logger.info("Upper {} {} already exists, skipped.", fav_item.upper.mid, fav_item.upper.name)
        elif any(isinstance(_, Exception) for _ in result):
            logger.exception("Failed to process upper {} {}.", fav_item.upper.mid, fav_item.upper.name)
    single_page = False
    if settings.paginated_video:
        pages = None
        try:
            pages = await v.get_pages()
            pages = [
                FavoriteItemPage(
                    favorite_item=fav_item,
                    cid=page["cid"],
                    page=page["page"],
                    name=page["part"],
                    image=page["first_frame"],
                )
                for page in pages
            ]
        except Exception:
            logger.exception("Failed to get pages of video {} {}.", fav_item.bvid, fav_item.name)
        if pages:
            if len(pages) == 1:
                single_page = True
            else:
                await FavoriteItemPage.bulk_create(
                    pages, on_conflict=["favorite_item_id", "page"], update_fields=["cid", "name", "image"]
                )
                if process_nfo:
                    pass

    if single_page or not settings.paginated_video:
        if process_nfo:
            try:
                await get_nfo(fav_item.nfo_path, obj=fav_item, mode=NfoMode.MOVIE)
            except FileExistsError:
                logger.info("NFO of {} {} already exists, skipped.", fav_item.bvid, fav_item.name)
            except Exception:
                logger.exception("Failed to process nfo of video {} {}", fav_item.bvid, fav_item.name)
        if process_poster:
            try:
                await get_file(fav_item.cover, fav_item.poster_path)
            except FileExistsError:
                logger.info("Poster of {} {} already exists, skipped.", fav_item.bvid, fav_item.name)
            except Exception:
                logger.exception("Failed to process poster of video {} {}", fav_item.bvid, fav_item.name)
        if process_subtitle:
            try:
                await get_subtitle(v, 0, fav_item.subtitle_path)
            except FileExistsError:
                logger.info("Subtitle of {} {} already exists, skipped.", fav_item.bvid, fav_item.name)
            except Exception:
                logger.exception("Failed to process subtitle of video {} {}", fav_item.bvid, fav_item.name)
        if process_video:
            try:
                await get_video(v, 0, fav_item.tmp_video_path, fav_item.tmp_audio_path, fav_item.video_path)
                fav_item.downloaded = True
            except Exception as e:
                errcode_status = {62002: MediaStatus.INVISIBLE, -404: MediaStatus.DELETED}
                if not (isinstance(e, ResponseCodeException) and (status := errcode_status.get(e.code))):
                    logger.exception("Failed to process video {} {}", fav_item.bvid, fav_item.name)
                else:
                    fav_item.status = status
                    logger.error(
                        "Video {} {} is not available, marked as {}", fav_item.bvid, fav_item.name, fav_item.status.text
                    )
    await fav_item.save()
    logger.info("{} {} has been processed.", fav_item.bvid, fav_item.name)


async def get_video(v: video.Video, page_id: int, tmp_video_path: Path, tmp_audio_path: Path, video_path: Path) -> None:
    """指定临时视频、音频和目标视频目录，下载视频的某个分p"""
    if await aexists(video_path):
        # 目标视频已经存在，忽略掉
        raise FileExistsError
    video_path.parent.mkdir(parents=True, exist_ok=True)
    # 分析对应分p的视频流
    detector = video.VideoDownloadURLDataDetecter(await v.get_download_url(page_index=page_id))
    streams = detector.detect_best_streams()
    if detector.check_flv_stream():
        # 对于 flv，直接下载
        await download_content(streams[0].url, tmp_video_path)
        process = await create_subprocess_exec(
            FFMPEG_COMMAND, "-i", tmp_video_path, video_path, stdout=DEVNULL, stderr=DEVNULL
        )
        await process.communicate()
        tmp_video_path.unlink()
    else:
        # 对于非 flv，首先要下载视频流
        paths, tasks = ([tmp_video_path], [download_content(streams[0].url, tmp_video_path)])
        if streams[1]:
            # 如果有音频流，也下载
            paths.append(tmp_audio_path)
            tasks.append(download_content(streams[1].url, tmp_audio_path))
        await asyncio.gather(*tasks)
        process = await create_subprocess_exec(
            FFMPEG_COMMAND,
            *sum([["-i", path] for path in paths], []),
            "-c",
            "copy",
            video_path,
            stdout=DEVNULL,
            stderr=DEVNULL,
        )
        await process.communicate()
        await asyncio.gather(*[aremove(path) for path in paths])


async def get_file(url: str, path: Path) -> None:
    """一个简单的下载封装，用于下载封面等内容"""
    if await aexists(path):
        # 目标文件已经存在，忽略掉
        raise FileExistsError
    path.parent.mkdir(parents=True, exist_ok=True)
    await download_content(url, path)


async def get_subtitle(v: video.Video, page_id: int, subtitle_path: Path) -> None:
    """指定目标字幕文件，下载视频的某个分p的字幕"""
    if await aexists(subtitle_path):
        # 目标字幕已经存在，忽略掉
        raise FileExistsError
    subtitle_path.parent.mkdir(parents=True, exist_ok=True)
    await ass.make_ass_file_danmakus_protobuf(
        v,
        page_id,
        str(subtitle_path.resolve()),
        credential=credential,
        font_name=settings.subtitle.font_name,
        font_size=settings.subtitle.font_size,
        alpha=settings.subtitle.alpha,
        fly_time=settings.subtitle.fly_time,
        static_time=settings.subtitle.static_time,
    )


async def get_nfo(nfo_path: Path, *, obj: Model, mode: NfoMode) -> None:
    """指定 nfo 路径、对象和模式，将对应的 nfo 信息写入到文件"""
    if await aexists(nfo_path):
        # 目标 nfo 已经存在，忽略掉
        raise FileExistsError
    nfo_path.parent.mkdir(parents=True, exist_ok=True)
    # 根据不同的模式，生成不同的 nfo
    nfo: NfoBase = None
    match obj, mode:
        case FavoriteItem(), NfoMode.MOVIE:
            nfo = MovieInfo.from_favorite_item(obj)
        case FavoriteItem(), NfoMode.TVSHOW:
            nfo = TVShowInfo.from_favorite_item(obj)
        case FavoriteItemPage(), NfoMode.EPISODE:
            nfo = EpisodeInfo.from_favorite_item_page(obj)
        case Upper(), NfoMode.UPPER:
            nfo = UpperInfo.from_upper(obj)
        case _:
            raise ValueError
    await nfo.to_file(nfo_path)
