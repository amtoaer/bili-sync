from tortoise import BaseDBAsyncClient


async def upgrade(db: BaseDBAsyncClient) -> str:
    return """
        CREATE TABLE IF NOT EXISTS "favoriteitempage" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "cid" INT NOT NULL,
    "page" INT NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "image" TEXT NOT NULL,
    "status" SMALLINT NOT NULL  DEFAULT 1 /* NORMAL: 1\nINVISIBLE: 2\nDELETED: 3 */,
    "downloaded" INT NOT NULL  DEFAULT 0,
    "favorite_item_id" INT NOT NULL REFERENCES "favoriteitem" ("id") ON DELETE CASCADE,
    CONSTRAINT "uid_favoriteite_favorit_c3b50e" UNIQUE ("favorite_item_id", "page")
) /* 收藏条目的分p */;"""


async def downgrade(db: BaseDBAsyncClient) -> str:
    return """
        DROP TABLE IF EXISTS "favoriteitempage";"""
