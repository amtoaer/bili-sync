import asyncio
import datetime
from asyncio import Semaphore, create_subprocess_exec
from asyncio.subprocess import DEVNULL

from bilibili_api import ass, favorite_list, video
from bilibili_api.exceptions import ResponseCodeException
from loguru import logger
from tortoise.connection import connections

from constants import FFMPEG_COMMAND, MediaStatus, MediaType
from credential import credential
from models import FavoriteItem, FavoriteList, Upper
from nfo import Actor, EpisodeInfo
from settings import settings
from utils import aexists, amakedirs, client, download_content

anchor = datetime.date.today()


async def cleanup() -> None:
    await client.aclose()
    await connections.close_all()


def concurrent_decorator(concurrency: int) -> callable:
    sem = Semaphore(value=concurrency)

    def decorator(func: callable) -> callable:
        async def wrapper(*args, **kwargs) -> any:
            async with sem:
                return await func(*args, **kwargs)

        return wrapper

    return decorator


async def manage_model(medias: list[dict], fav_list: FavoriteList) -> None:
    uppers = [
        Upper(
            mid=media["upper"]["mid"],
            name=media["upper"]["name"],
            thumb=media["upper"]["face"],
        )
        for media in medias
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
    logger.info(
        "Start to process favorite {}: {}",
        favorite_id,
        title,
    )
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
            favorite_list=fav_list,
            bvid__in=[media["bvid"] for media in favorite_video_list["medias"]],
        )
        # 记录一下获得的列表中的 bvid 和 fav_time
        media_info = {(media["bvid"], media["fav_time"]) for media in favorite_video_list["medias"]}
        # 如果有 bvid 和 fav_time 都相同的记录，说明已经到达了上次处理到的位置
        continue_flag = not media_info & {
            (item.bvid, int(item.fav_time.timestamp())) for item in existed_items
        }
        await manage_model(favorite_video_list["medias"], fav_list)
        if not (continue_flag and favorite_video_list["has_more"]):
            break
    all_unprocessed_items = await FavoriteItem.filter(
        favorite_list=fav_list,
        type=MediaType.VIDEO,
        status=MediaStatus.NORMAL,
        downloaded=False,
    ).prefetch_related("upper")
    await asyncio.gather(
        *[process_favorite_item(item) for item in all_unprocessed_items],
        return_exceptions=True,
    )
    logger.info("Favorite {} {} processed successfully.", favorite_id, title)


@concurrent_decorator(4)
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
    # 如果没有获取过 tags，那么尝试获取一下
    try:
        if fav_item.tags is None:
            fav_item.tags = [_["tag_name"] for _ in await v.get_tags()]
    except Exception:
        logger.exception(
            "Failed to get tags of video {} {}",
            fav_item.bvid,
            fav_item.name,
        )

    if process_upper:
        try:
            if not all(
                await asyncio.gather(
                    aexists(fav_item.upper.thumb_path),
                    aexists(fav_item.upper.meta_path),
                )
            ):
                await amakedirs(fav_item.upper.thumb_path.parent, exist_ok=True)
                await asyncio.gather(
                    fav_item.upper.save_metadata(),
                    download_content(fav_item.upper.thumb, fav_item.upper.thumb_path),
                    return_exceptions=True,
                )
            else:
                logger.info(
                    "Upper {} {} already exists, skipped.",
                    fav_item.upper.mid,
                    fav_item.upper.name,
                )
        except Exception:
            logger.exception(
                "Failed to process upper {} {}",
                fav_item.upper.mid,
                fav_item.upper.name,
            )

    if process_nfo:
        try:
            if not await aexists(fav_item.nfo_path):
                await EpisodeInfo(
                    title=fav_item.name,
                    plot=fav_item.desc,
                    actor=[
                        Actor(
                            name=fav_item.upper.mid,
                            role=fav_item.upper.name,
                        )
                    ],
                    tags=fav_item.tags,
                    bvid=fav_item.bvid,
                    aired=fav_item.ctime,
                ).write_nfo(fav_item.nfo_path)
            else:
                logger.info(
                    "NFO of {} {} already exists, skipped.",
                    fav_item.bvid,
                    fav_item.name,
                )
        except Exception:
            logger.exception(
                "Failed to process nfo of video {} {}",
                fav_item.bvid,
                fav_item.name,
            )

    if process_poster:
        try:
            if not await aexists(fav_item.poster_path):
                try:
                    await download_content(fav_item.cover, fav_item.poster_path)
                except Exception:
                    logger.exception(
                        "Failed to download poster of video {} {}",
                        fav_item.bvid,
                        fav_item.name,
                    )
            else:
                logger.info(
                    "Poster of {} {} already exists, skipped.",
                    fav_item.bvid,
                    fav_item.name,
                )
        except Exception:
            logger.exception(
                "Failed to process poster of video {} {}",
                fav_item.bvid,
                fav_item.name,
            )

    if process_subtitle:
        try:
            if not await aexists(fav_item.subtitle_path):
                await ass.make_ass_file_danmakus_protobuf(
                    v,
                    0,
                    str(fav_item.subtitle_path.resolve()),
                    credential=credential,
                    font_name=settings.subtitle.font_name,
                    font_size=settings.subtitle.font_size,
                    alpha=settings.subtitle.alpha,
                    fly_time=settings.subtitle.fly_time,
                    static_time=settings.subtitle.static_time,
                )
            else:
                logger.info(
                    "Subtitle of {} {} already exists, skipped.",
                    fav_item.bvid,
                    fav_item.name,
                )
        except Exception:
            logger.exception(
                "Failed to process subtitle of video {} {}",
                fav_item.bvid,
                fav_item.name,
            )
    if process_video:
        try:
            if await aexists(fav_item.video_path):
                fav_item.downloaded = True
                logger.info(
                    "Video {} {} already exists, skipped.",
                    fav_item.bvid,
                    fav_item.name,
                )
            else:
                # 开始处理视频内容
                detector = video.VideoDownloadURLDataDetecter(
                    await v.get_download_url(page_index=0)
                )
                streams = detector.detect_best_streams(codecs=settings.codec)
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
        except ResponseCodeException as e:
            match e.code:
                case 62002:
                    fav_item.status = MediaStatus.INVISIBLE
                case -404:
                    fav_item.status = MediaStatus.DELETED
                case _:
                    logger.exception(
                        "Failed to process video {} {}, error_code: {}",
                        fav_item.bvid,
                        fav_item.name,
                        e.code,
                    )
            if fav_item.status != MediaStatus.NORMAL:
                logger.error(
                    "Video {} {} is not available, marked as {}",
                    fav_item.bvid,
                    fav_item.name,
                    fav_item.status.text,
                )
        except Exception:
            logger.exception("Failed to process video {} {}", fav_item.bvid, fav_item.name)
    await fav_item.save()
    logger.info(
        "{} {} is processed successfully.",
        fav_item.bvid,
        fav_item.name,
    )
