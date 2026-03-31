import contextlib
import datetime
import http.cookiejar
import json
import os
import re
import requests
import urllib.parse
import xml.etree.ElementTree as ET

try:
    import yt_dlp
except ImportError:
    yt_dlp = None


ILLEGAL_XML_CHARS_RE = re.compile(r"[\x00-\x08\x0B\x0C\x0E-\x1F]")
YOUTUBE_HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
    ),
    "Accept-Language": "en-US,en;q=0.9",
}


def ensure_ytdlp():
    if yt_dlp is None:
        raise RuntimeError("yt-dlp is not installed")


def clean_xml_text(value):
    if value is None:
        return ""
    text = str(value).replace("\r\n", "\n").replace("\r", "\n")
    return ILLEGAL_XML_CHARS_RE.sub("", text)


def indent_xml(element, level=0):
    indent = "\n" + "  " * level
    child_indent = "\n" + "  " * (level + 1)
    children = list(element)
    if children:
        if not element.text or not element.text.strip():
            element.text = child_indent
        for child in children:
            indent_xml(child, level + 1)
        if not children[-1].tail or not children[-1].tail.strip():
            children[-1].tail = indent
    if level and (not element.tail or not element.tail.strip()):
        element.tail = indent


def add_text_element(parent, tag, value):
    text = clean_xml_text(value)
    if text:
        ET.SubElement(parent, tag).text = text
        return True
    return False


def description_outline(text, max_len=180):
    content = clean_xml_text(text)
    if not content:
        return ""
    first_line = next((line.strip() for line in content.splitlines() if line.strip()), "")
    return first_line[:max_len].rstrip()


def sanitize_filename(title):
    sanitized = "".join(c for c in str(title) if c not in '/\\\0').strip()
    max_bytes = 200
    while len(sanitized.encode("utf-8")) > max_bytes - 3 and sanitized:
        sanitized = sanitized[:-1]
    sanitized = sanitized.rstrip(". ")
    return sanitized or "video"


def canonical_video_url(video_id):
    return f"https://www.youtube.com/watch?v={video_id}"


def canonical_playlist_url(playlist_id):
    return f"https://www.youtube.com/playlist?list={playlist_id}"


def extract_playlist_id_from_url(url):
    parsed = urllib.parse.urlparse(clean_xml_text(url).strip())
    query = urllib.parse.parse_qs(parsed.query or "")
    playlist_ids = query.get("list", [])
    return playlist_ids[0] if playlist_ids else ""


def normalize_youtube_url(url):
    text = clean_xml_text(url).strip()
    if not text:
        return text
    parsed = urllib.parse.urlparse(text)
    host = parsed.netloc.lower()
    path = parsed.path or ""
    if "youtu.be" in host:
        video_id = path.lstrip("/").split("/")[0]
        if video_id:
            return canonical_video_url(video_id)
    if "youtube.com" in host:
        query = urllib.parse.parse_qs(parsed.query or "")
        if query.get("v"):
            return canonical_video_url(query["v"][0])
        match = re.match(r"^/(?:shorts|live|embed)/([^/?#]+)", path)
        if match:
            return canonical_video_url(match.group(1))
    return text


def detect_youtube_url_kind(url):
    text = clean_xml_text(url).strip()
    if not text:
        raise RuntimeError("link is empty")
    parsed = urllib.parse.urlparse(text)
    host = parsed.netloc.lower()
    path = (parsed.path or "").rstrip("/")
    query = urllib.parse.parse_qs(parsed.query or "")
    if "youtu.be" in host:
        return "video"
    if "youtube.com" not in host:
        raise RuntimeError("仅支持 YouTube 链接")
    if query.get("list"):
        return "playlist"
    if re.match(r"^/(?:watch|shorts|live|embed)(?:/|$)", path):
        return "video"
    if path.endswith("/playlists"):
        return "playlist_collection"
    if path.startswith("/playlist"):
        return "playlist"
    if re.match(r"^/(?:channel|c|user)/", path) or path.startswith("/@"):
        return "channel"
    return "video"


def build_ytdlp_base_opts():
    return {
        "quiet": True,
        "no_warnings": True,
        "extract_flat": False,
        "ignore_no_formats_error": True,
        "force_ipv4": True,
        "socket_timeout": 60,
        "retries": 10,
        "http_headers": dict(YOUTUBE_HEADERS),
        "js_runtimes": {
            "deno": {},
            "node": {"path": "node"},
        },
        "remote_components": {"ejs:github"},
    }


def build_ytdlp_opts(cookie_file=None, extract_flat=False, playlist_end=None):
    opts = build_ytdlp_base_opts()
    if extract_flat:
        opts["extract_flat"] = True
    if playlist_end:
        opts["playlistend"] = playlist_end
    if cookie_file:
        opts["cookiefile"] = os.path.expandvars(cookie_file.strip('"'))
    return opts


def extract_with_ytdlp(url, cookie_file=None, extract_flat=False, playlist_end=None):
    ensure_ytdlp()
    opts = build_ytdlp_opts(cookie_file, extract_flat=extract_flat, playlist_end=playlist_end)
    with yt_dlp.YoutubeDL(opts) as ydl:
        return ydl.extract_info(url, download=False)


def extract_with_cookie_fallback(url, cookie_file=None, extract_flat=False, playlist_end=None):
    candidates = [cookie_file] if cookie_file else [None]
    if cookie_file:
        candidates.append(None)
    last_error = None
    for candidate in candidates:
        try:
            return extract_with_ytdlp(
                url,
                candidate,
                extract_flat=extract_flat,
                playlist_end=playlist_end,
            )
        except Exception as error:
            last_error = error
    raise last_error or RuntimeError("yt-dlp extract failed")


def has_playable_formats(formats):
    if not formats:
        return False
    for item in formats:
        ext = (item.get("ext") or "").lower()
        vcodec = (item.get("vcodec") or "").lower()
        acodec = (item.get("acodec") or "").lower()
        if ext == "mhtml" or vcodec == "images":
            continue
        if vcodec != "none" or (acodec and acodec != "none"):
            return True
    return False


def get_video_info(url, cookie_file=None):
    ensure_ytdlp()
    url = normalize_youtube_url(url)
    opts = build_ytdlp_base_opts()
    if cookie_file:
        opts["cookiefile"] = os.path.expandvars(cookie_file.strip('"'))
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)
    if not info:
        raise RuntimeError("failed to get video info")
    if info.get("_type") == "playlist":
        entries = info.get("entries") or []
        info = next((entry for entry in entries if entry), None)
        if not info:
            raise RuntimeError("failed to resolve playlist video")
    formats = info.get("formats", [])
    if cookie_file and not has_playable_formats(formats):
        retry_opts = build_ytdlp_base_opts()
        with yt_dlp.YoutubeDL(retry_opts) as ydl:
            retry_info = ydl.extract_info(url, download=False)
        if retry_info and retry_info.get("_type") == "playlist":
            entries = retry_info.get("entries") or []
            retry_info = next((entry for entry in entries if entry), None)
        retry_formats = (retry_info or {}).get("formats", [])
        if retry_info and has_playable_formats(retry_formats):
            info = retry_info
            formats = retry_formats
            opts.pop("cookiefile", None)
    if not has_playable_formats(formats):
        raise RuntimeError("no playable formats detected")
    upload_date = info.get("upload_date", "")
    publish_date = ""
    year = ""
    if upload_date and len(upload_date) == 8:
        publish_date = f"{upload_date[:4]}-{upload_date[4:6]}-{upload_date[6:8]}"
        year = upload_date[:4]
    return {
        "title": sanitize_filename(info.get("title", "video")),
        "description": info.get("description", ""),
        "uploader": info.get("uploader") or info.get("channel") or "YouTube",
        "publish_date": publish_date,
        "year": year,
        "thumbnail_url": info.get("thumbnail", ""),
        "tags": info.get("tags", []),
        "url": url,
        "formats": formats,
        "cookiefile": opts.get("cookiefile"),
        "original_info": info,
    }


def download_video(info, output_dir, output_format):
    ensure_ytdlp()
    base_opts = build_ytdlp_base_opts()
    base_opts.update(
        {
            "merge_output_format": output_format,
            "remuxvideo": output_format,
            "outtmpl": os.path.join(output_dir, f"{sanitize_filename(info['title'])}.%(ext)s"),
            "prefer_ffmpeg": True,
        }
    )
    cookie_candidates = [info.get("cookiefile")]
    if info.get("cookiefile"):
        cookie_candidates.append(None)
    format_candidates = [
        "bestvideo*+bestaudio/best",
        "bestvideo+bestaudio/best",
        "best",
    ]
    last_error = None
    for cookie_candidate in cookie_candidates:
        for format_expr in format_candidates:
            try:
                opts = dict(base_opts)
                opts["format"] = format_expr
                opts["cookiefile"] = cookie_candidate
                with yt_dlp.YoutubeDL(opts) as ydl:
                    ydl.download([info["url"]])
                last_error = None
                break
            except Exception as error:
                last_error = error
        if last_error is None:
            break
    if last_error is not None:
        raise last_error
    downloaded_files = [
        filename
        for filename in os.listdir(output_dir)
        if filename.startswith(sanitize_filename(info["title"]))
        and os.path.splitext(filename)[1].lower() in {".mp4", ".mkv", ".webm", ".mov", ".m4v"}
    ]
    if not downloaded_files:
        raise RuntimeError("video file was not produced")
    preferred = [filename for filename in downloaded_files if filename.lower().endswith(f".{output_format}")]
    final_file = preferred[0] if preferred else downloaded_files[0]
    cleanup_intermediate_files(info, output_dir, final_file)
    return final_file


def download_subtitles(info, output_dir):
    ensure_ytdlp()
    opts = build_ytdlp_base_opts()
    opts.update(
        {
            "writesubtitles": True,
            "subtitleslangs": ["ja", "zh-Hans", "zh-Hant", "en"],
            "subtitlesformat": "ass/srt/vtt",
            "skip_download": True,
            "cookiefile": info.get("cookiefile"),
            "outtmpl": os.path.join(output_dir, f"{sanitize_filename(info['title'])}.%(ext)s"),
        }
    )
    with yt_dlp.YoutubeDL(opts) as ydl:
        with contextlib.suppress(Exception):
            ydl.download([info["url"]])


def generate_metadata_files(video_info, output_dir, skip_poster=False, skip_nfo=False):
    base_name = os.path.splitext(video_info["title"])[0]
    thumbnail_path = os.path.join(output_dir, f"{base_name}-poster.jpg")

    if not skip_poster and video_info.get("thumbnail_url"):
        response = requests.get(video_info["thumbnail_url"], headers=YOUTUBE_HEADERS, timeout=15)
        response.raise_for_status()
        with open(thumbnail_path, "wb") as file:
            file.write(response.content)

    if skip_nfo:
        return

    original = video_info.get("original_info") or {}
    video_id = original.get("id", "")
    webpage_url = original.get("webpage_url") or video_info.get("url", "")
    duration_seconds = original.get("duration")
    runtime_minutes = ""
    if isinstance(duration_seconds, (int, float)) and duration_seconds > 0:
        runtime_minutes = str(max(1, int(round(duration_seconds / 60.0))))

    root = ET.Element("movie")
    add_text_element(root, "title", video_info.get("title"))
    add_text_element(root, "originaltitle", video_info.get("title"))
    add_text_element(root, "plot", video_info.get("description"))
    add_text_element(root, "outline", description_outline(video_info.get("description")))
    add_text_element(root, "year", video_info.get("year"))
    add_text_element(root, "premiered", video_info.get("publish_date"))
    add_text_element(root, "aired", video_info.get("publish_date"))
    add_text_element(root, "runtime", runtime_minutes)
    add_text_element(root, "studio", "YouTube")
    add_text_element(root, "trailer", webpage_url)
    add_text_element(root, "id", video_id)
    if clean_xml_text(video_id):
        uniqueid = ET.SubElement(root, "uniqueid", {"type": "youtube", "default": "true"})
        uniqueid.text = clean_xml_text(video_id)
    if os.path.exists(thumbnail_path):
        add_text_element(root, "thumb", os.path.basename(thumbnail_path))
    add_text_element(root, "genre", "YouTube")
    actor = ET.SubElement(root, "actor")
    ET.SubElement(actor, "name").text = clean_xml_text(video_info.get("uploader"))
    for tag in video_info.get("tags", [])[:10]:
        if clean_xml_text(tag):
            ET.SubElement(root, "tag").text = clean_xml_text(tag)
    nfo_path = os.path.join(output_dir, f"{base_name}.nfo")
    indent_xml(root)
    ET.ElementTree(root).write(
        nfo_path,
        encoding="utf-8",
        xml_declaration=True,
        short_empty_elements=False,
    )


def session_from_cookie_file(cookie_file):
    cookie_jar = http.cookiejar.MozillaCookieJar()
    cookie_jar.load(cookie_file, ignore_discard=True, ignore_expires=True)
    session = requests.Session()
    session.headers.update(dict(YOUTUBE_HEADERS))
    for cookie in cookie_jar:
        session.cookies.set_cookie(cookie)
    return session


def parse_channel_id_from_feed_url(feed_url):
    parsed = urllib.parse.urlparse(feed_url)
    query = urllib.parse.parse_qs(parsed.query or "")
    channel_ids = query.get("channel_id", [])
    return channel_ids[0] if channel_ids else ""


def list_subscriptions(cookie_file):
    session = session_from_cookie_file(cookie_file)
    takeout_url = "https://www.youtube.com/subscription_manager?action_takeout=1"
    response = session.get(takeout_url, timeout=30)
    response.raise_for_status()

    content_type = response.headers.get("content-type", "")
    text = response.text
    if "xml" in content_type or "<opml" in text.lower():
        channels = parse_subscription_takeout(text)
        if channels:
            return channels

    channels = parse_feed_channels_html(text)
    if channels:
        return channels

    raise RuntimeError("failed to extract subscribed channels from YouTube")


def list_playlists(cookie_file):
    info = extract_with_ytdlp(
        "https://www.youtube.com/feed/playlists",
        cookie_file,
        extract_flat=True,
    )
    entries = info.get("entries") or []
    playlists = []
    seen = set()
    for entry in entries:
        if not entry:
            continue
        playlist_id = entry.get("id") or entry.get("playlist_id") or extract_playlist_id_from_url(
            entry.get("url") or entry.get("webpage_url") or ""
        )
        title = clean_xml_text(entry.get("title", "")).strip()
        if not playlist_id or not title or playlist_id in seen:
            continue
        seen.add(playlist_id)
        playlists.append(
            {
                "playlist_id": playlist_id,
                "name": title,
                "url": ensure_absolute_youtube_url(entry.get("url"))
                or ensure_absolute_youtube_url(entry.get("webpage_url"))
                or canonical_playlist_url(playlist_id),
                "thumbnail": extract_thumbnail_any(entry),
                "owner_name": entry.get("channel") or entry.get("uploader"),
                "video_count": normalize_count(entry.get("playlist_count") or entry.get("n_entries")),
            }
        )
    playlists.sort(key=lambda item: item["name"].lower())
    return playlists


def resolve_source(url, cookie_file=None):
    kind = detect_youtube_url_kind(url)
    if kind == "playlist_collection":
        raise RuntimeError("请提交具体播放列表链接，而不是播放列表页面")
    if kind == "video":
        info = get_video_info(url, cookie_file)
        original = info.get("original_info") or {}
        return {
            "kind": "video",
            "source_id": original.get("id", ""),
            "name": info.get("title", "YouTube"),
            "url": original.get("webpage_url") or info.get("url"),
            "thumbnail": info.get("thumbnail_url") or None,
            "owner_name": info.get("uploader"),
            "video_count": None,
        }

    info = extract_with_cookie_fallback(url, cookie_file, extract_flat=True, playlist_end=1)
    if not info:
        raise RuntimeError("failed to resolve YouTube link")

    if kind == "playlist":
        playlist_id = info.get("id") or info.get("playlist_id") or extract_playlist_id_from_url(
            info.get("webpage_url") or url
        )
        if not playlist_id:
            raise RuntimeError("failed to resolve playlist id")
        return {
            "kind": "playlist",
            "source_id": playlist_id,
            "name": clean_xml_text(info.get("title") or playlist_id).strip(),
            "url": ensure_absolute_youtube_url(info.get("webpage_url")) or canonical_playlist_url(playlist_id),
            "thumbnail": extract_thumbnail_any(info),
            "owner_name": info.get("channel") or info.get("uploader"),
            "video_count": normalize_count(info.get("playlist_count") or info.get("n_entries")),
        }

    channel_id = info.get("channel_id")
    if not channel_id:
        entries = info.get("entries") or []
        first = next((entry for entry in entries if entry), {}) if entries else {}
        channel_id = first.get("channel_id") or ""
    if not channel_id:
        raise RuntimeError("failed to resolve channel id")
    channel_name = (
        clean_xml_text(info.get("channel") or info.get("uploader") or info.get("title") or channel_id).strip()
    )
    channel_url = (
        ensure_absolute_youtube_url(info.get("uploader_url"))
        or ensure_absolute_youtube_url(info.get("channel_url"))
        or ensure_absolute_youtube_url(info.get("webpage_url"))
        or f"https://www.youtube.com/channel/{channel_id}"
    )
    return {
        "kind": "channel",
        "source_id": channel_id,
        "name": channel_name,
        "url": channel_url,
        "thumbnail": extract_thumbnail_any(info),
        "owner_name": info.get("channel") or info.get("uploader"),
        "video_count": normalize_count(info.get("playlist_count") or info.get("n_entries")),
    }


def list_playlist_videos(url, cookie_file=None):
    info = extract_with_cookie_fallback(url, cookie_file, extract_flat=True)
    entries = info.get("entries") or []
    uploader = info.get("channel") or info.get("uploader") or info.get("title") or "YouTube"
    videos = []
    seen = set()
    for entry in entries:
        if not entry:
            continue
        video_id = entry.get("id")
        title = clean_xml_text(entry.get("title", "")).strip()
        if not video_id or not title or video_id in seen:
            continue
        seen.add(video_id)
        videos.append(
            {
                "video_id": video_id,
                "title": title,
                "url": canonical_video_url(video_id),
                "description": entry.get("description") or "",
                "uploader": entry.get("channel") or entry.get("uploader") or uploader,
                "thumbnail": extract_thumbnail_any(entry),
                "published_at": parse_entry_timestamp(entry),
            }
        )
    return videos


def parse_subscription_takeout(text):
    root = ET.fromstring(text)
    channels = []
    for outline in root.findall(".//outline"):
        xml_url = outline.attrib.get("xmlUrl", "")
        html_url = outline.attrib.get("htmlUrl", "")
        title = outline.attrib.get("title") or outline.attrib.get("text") or ""
        channel_id = parse_channel_id_from_feed_url(xml_url)
        if channel_id and title:
            channels.append(
                {
                    "channel_id": channel_id,
                    "name": title,
                    "url": html_url or f"https://www.youtube.com/channel/{channel_id}",
                    "thumbnail": None,
                }
            )
    channels.sort(key=lambda item: item["name"].lower())
    return channels


def parse_feed_channels_html(text):
    match = re.search(r"ytInitialData\s*=\s*(\{.*?\})\s*;</script>", text, re.S)
    if not match:
        return []

    data = json.loads(match.group(1))
    renderers = []
    walk_json(data, renderers)

    channels = []
    seen = set()
    for item in renderers:
        renderer = item.get("channelRenderer") or item.get("gridChannelRenderer")
        if not renderer:
            continue
        channel_id = renderer.get("channelId", "")
        title = extract_runs_text(renderer.get("title")) or renderer.get("title", {}).get("simpleText", "")
        endpoint = renderer.get("navigationEndpoint", {}).get("browseEndpoint", {}).get("canonicalBaseUrl")
        url = f"https://www.youtube.com{endpoint}" if endpoint else f"https://www.youtube.com/channel/{channel_id}"
        thumbnail = extract_thumbnail_url(renderer.get("thumbnail"))
        if channel_id and title and channel_id not in seen:
            seen.add(channel_id)
            channels.append(
                {
                    "channel_id": channel_id,
                    "name": title,
                    "url": url,
                    "thumbnail": thumbnail,
                }
            )
    channels.sort(key=lambda item: item["name"].lower())
    return channels


def walk_json(value, renderers):
    if isinstance(value, dict):
        if "channelRenderer" in value or "gridChannelRenderer" in value:
            renderers.append(value)
        for child in value.values():
            walk_json(child, renderers)
    elif isinstance(value, list):
        for child in value:
            walk_json(child, renderers)


def extract_runs_text(value):
    if not isinstance(value, dict):
        return ""
    runs = value.get("runs")
    if not isinstance(runs, list):
        return ""
    return "".join(part.get("text", "") for part in runs)


def extract_thumbnail_url(value):
    if not isinstance(value, dict):
        return None
    thumbnails = value.get("thumbnails")
    if isinstance(thumbnails, list) and thumbnails:
        return thumbnails[-1].get("url")
    return None


def extract_thumbnail_any(value):
    if isinstance(value, dict):
        return (
            value.get("thumbnail")
            or extract_thumbnail_url(value)
            or extract_thumbnail_any(value.get("thumbnails"))
        )
    if isinstance(value, list) and value:
        last = value[-1]
        if isinstance(last, dict):
            return last.get("url")
    if isinstance(value, str) and value.strip():
        return value
    return None


def ensure_absolute_youtube_url(url):
    text = clean_xml_text(url).strip()
    if not text:
        return None
    if text.startswith("http://") or text.startswith("https://"):
        return text
    if text.startswith("/"):
        return f"https://www.youtube.com{text}"
    return None


def normalize_count(value):
    if isinstance(value, int):
        return value
    if isinstance(value, str) and value.isdigit():
        return int(value)
    return None


def parse_entry_timestamp(entry):
    timestamp = entry.get("timestamp") or entry.get("release_timestamp")
    if isinstance(timestamp, (int, float)):
        return int(timestamp)
    upload_date = entry.get("upload_date")
    if isinstance(upload_date, str) and len(upload_date) == 8 and upload_date.isdigit():
        dt = datetime.datetime(
            int(upload_date[:4]),
            int(upload_date[4:6]),
            int(upload_date[6:8]),
            tzinfo=datetime.timezone.utc,
        )
        return int(dt.timestamp())
    return None


def cleanup_intermediate_files(info, output_dir, final_file):
    base_name = sanitize_filename(info["title"])
    temp_pattern = re.compile(rf"^{re.escape(base_name)}\.f\d+\.[^.]+$")
    for filename in os.listdir(output_dir):
        if filename == final_file:
            continue
        if temp_pattern.match(filename) or filename.startswith(f"{base_name}.part"):
            with contextlib.suppress(FileNotFoundError):
                os.remove(os.path.join(output_dir, filename))
