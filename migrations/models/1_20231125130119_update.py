from tortoise import BaseDBAsyncClient


async def upgrade(db: BaseDBAsyncClient) -> str:
    return """
        ALTER TABLE "favoriteitem" ADD "status" SMALLINT NOT NULL  DEFAULT 1 /* NORMAL: 1\nINVISIBLE: 2\nDELETED: 3 */;"""


async def downgrade(db: BaseDBAsyncClient) -> str:
    return """
        ALTER TABLE "favoriteitem" DROP COLUMN "status";"""
