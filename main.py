import asyncio
from processor import process


async def main() -> None:
    await process()


if __name__ == "__main__":
    asyncio.run(main())
