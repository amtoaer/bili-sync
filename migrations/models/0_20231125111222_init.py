from tortoise import BaseDBAsyncClient


async def upgrade(db: BaseDBAsyncClient) -> str:
    return """
        CREATE TABLE IF NOT EXISTS "favoritelist" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "created_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP
) /* 收藏列表 */;
CREATE TABLE IF NOT EXISTS "upper" (
    "mid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "thumb" TEXT NOT NULL,
    "created_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP
) /* up主 */;
CREATE TABLE IF NOT EXISTS "favoriteitem" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "type" SMALLINT NOT NULL  /* VIDEO: 2\nAUDIO: 12\nVIDEO_COLLECTION: 21 */,
    "bvid" VARCHAR(255) NOT NULL,
    "desc" TEXT NOT NULL,
    "cover" TEXT NOT NULL,
    "ctime" TIMESTAMP NOT NULL,
    "pubtime" TIMESTAMP NOT NULL,
    "fav_time" TIMESTAMP NOT NULL,
    "downloaded" INT NOT NULL  DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    "favorite_list_id" INT NOT NULL REFERENCES "favoritelist" ("id") ON DELETE CASCADE,
    "upper_id" INT NOT NULL REFERENCES "upper" ("mid") ON DELETE CASCADE,
    CONSTRAINT "uid_favoriteite_bvid_d7b8ea" UNIQUE ("bvid", "favorite_list_id")
) /* 收藏条目 */;
CREATE TABLE IF NOT EXISTS "aerich" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "version" VARCHAR(255) NOT NULL,
    "app" VARCHAR(100) NOT NULL,
    "content" JSON NOT NULL
);"""


async def downgrade(db: BaseDBAsyncClient) -> str:
    return """
        """
