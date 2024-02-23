from bilibili_api import Credential

from settings import settings


class PersistedCredential(Credential):
    def __init__(self) -> None:
        super().__init__(
            settings.sessdata, settings.bili_jct, settings.buvid3, settings.dedeuserid, settings.ac_time_value
        )

    async def refresh(self) -> None:
        await super().refresh()
        (settings.sessdata, settings.bili_jct, settings.dedeuserid, settings.ac_time_value) = (
            self.sessdata,
            self.bili_jct,
            self.dedeuserid,
            self.ac_time_value,
        )
        await settings.asave()


credential = PersistedCredential()
