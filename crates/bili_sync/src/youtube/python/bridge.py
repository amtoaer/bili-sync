import argparse
import contextlib
import json
import os
import sys

from core import (
    download_subtitles,
    download_video,
    generate_metadata_files,
    get_video_info,
    list_playlist_videos,
    list_playlists,
    list_subscriptions,
    resolve_source,
)


def command_subscriptions(args):
    return list_subscriptions(args.cookie_file)


def command_playlists(args):
    return list_playlists(args.cookie_file)


def command_resolve_source(args):
    return resolve_source(args.url, args.cookie_file)


def command_playlist_videos(args):
    return list_playlist_videos(args.url, args.cookie_file)


def command_download(args):
    info = get_video_info(args.url, args.cookie_file)
    final_output_dir = os.path.join(args.output_dir, info["title"])
    os.makedirs(final_output_dir, exist_ok=True)
    video_file = download_video(info, final_output_dir, args.output_format)
    if not args.skip_subtitle:
        with contextlib.suppress(Exception):
            download_subtitles(info, final_output_dir)
    with contextlib.suppress(Exception):
        generate_metadata_files(
            info,
            final_output_dir,
            skip_poster=args.skip_poster,
            skip_nfo=args.skip_nfo,
        )
    return {
        "output_dir": final_output_dir,
        "video_file": video_file,
    }


def build_parser():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    subscriptions = subparsers.add_parser("subscriptions")
    subscriptions.add_argument("--cookie-file", required=True)
    subscriptions.set_defaults(handler=command_subscriptions)

    playlists = subparsers.add_parser("playlists")
    playlists.add_argument("--cookie-file", required=True)
    playlists.set_defaults(handler=command_playlists)

    resolve_source_parser = subparsers.add_parser("resolve-source")
    resolve_source_parser.add_argument("--url", required=True)
    resolve_source_parser.add_argument("--cookie-file")
    resolve_source_parser.set_defaults(handler=command_resolve_source)

    playlist_videos = subparsers.add_parser("playlist-videos")
    playlist_videos.add_argument("--url", required=True)
    playlist_videos.add_argument("--cookie-file")
    playlist_videos.set_defaults(handler=command_playlist_videos)

    download = subparsers.add_parser("download")
    download.add_argument("--url", required=True)
    download.add_argument("--output-dir", required=True)
    download.add_argument("--output-format", choices=["mp4", "mkv", "webm"], default="mp4")
    download.add_argument("--cookie-file")
    download.add_argument("--skip-poster", action="store_true")
    download.add_argument("--skip-nfo", action="store_true")
    download.add_argument("--skip-subtitle", action="store_true")
    download.set_defaults(handler=command_download)

    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()
    try:
        with contextlib.redirect_stdout(sys.stderr):
            result = args.handler(args)
        if result is not None:
            print(json.dumps(result, ensure_ascii=False))
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
