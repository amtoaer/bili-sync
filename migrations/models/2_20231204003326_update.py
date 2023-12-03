from tortoise import BaseDBAsyncClient


async def upgrade(db: BaseDBAsyncClient) -> str:
    return """
        ALTER TABLE "favoriteitem" ADD "tags" JSON;"""


async def downgrade(db: BaseDBAsyncClient) -> str:
    return """
        ALTER TABLE "favoriteitem" DROP COLUMN "tags";"""
