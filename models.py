from tortoise import Tortoise, fields
from tortoise.models import Model

from constants import DEFAULT_DATABASE_PATH


class FavoriteList(Model):
    """收藏列表"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)


class Upper(Model):
    """up主"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)


class FavoriteItem(Model):
    """收藏条目"""

    id = fields.IntField(pk=True)
    name = fields.CharField(max_length=255)
    bvid = fields.CharField(max_length=255)
    favorite_list = fields.ForeignKeyField(
        "models.FavoriteList", related_name="items"
    )
    upper = fields.ForeignKeyField("models.Upper", related_name="uploads")
    created_at = fields.DatetimeField(auto_now_add=True)
    updated_at = fields.DatetimeField(auto_now=True)


async def init_model():
    await Tortoise.init(
        db_url=f"sqlite://{DEFAULT_DATABASE_PATH}",
        modules={"models": ["models"]},
    )
    await Tortoise.generate_schemas()
