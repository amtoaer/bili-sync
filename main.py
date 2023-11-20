import asyncio

from bilibili_api import favorite_list


async def main() -> None:
    result = await favorite_list.get_video_favorite_list(9183758)
    print(result)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
