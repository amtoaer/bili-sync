# bili-sync

为 NAS 用户编写的 BILIBILI 收藏夹同步工具，可方便导入 EMBY 等媒体库工具浏览。

> 经常在点进自己的收藏夹时看到一大堆失效视频，简直逼死强迫症。想到自己 NAS 还有多余的 20T 存储空间，于是简单写了一个小工具用来实时把收藏夹内容同步下载到 NAS 上。
> ![](asset/space.png)

骄傲地由 python 驱动！

~~之前基本没有用过 python 的 asyncio，拿这个小工具练练手！（不是~~

## 工作截图

![下载视频](asset/run.png)

![EMBY 识别](asset/emby.png)

## 配置文件

对于配置文件的前五项，请参考[凭据获取流程](https://nemo2011.github.io/bilibili-api/#/get-credential)。

```python
class Config(DataClassJsonMixin):
    sessdata: str = ""
    bili_jct: str = ""
    buvid3: str = ""
    dedeuserid: str = ""
    ac_time_value: str = ""
    interval: int = 20    # 任务执行的间隔时间
    favorite_ids: list[int] = field(default_factory=list)  # 收藏夹的 id
    path_mapper: dict[int, str] = field(default_factory=dict)  # 收藏夹的 id 到存储目录的映射
```

程序默认会将配置文件存储于 `${程序路径}/config/config.json`，数据库文件存储于 `${程序路径}/data/data.db`，如果发现不存在则新建并写入初始配置。

配置文件加载时会校验内容是否为空，对于默认的空配置，校验将会报错，程序将会终止运行。

即：我们可以通过运行一次程序，等程序写入初始配置并提示配置错误终止后编辑 `config.json` 文件，编辑后即可重新运行。

## 关于 UP 头像

目前开放全局的环境变量 `THUMB_PATH` 作为 up 主头像的存储位置。

在下载某条视频时，如果 UP 的头像还不存在，就会将 UP 的头像下载至 `THUMB_PATH`，同时在视频的 NFO 文件中写入 UP 头像的绝对路径。

但实际测试下来，EMBY 似乎无法正常读取 NFO 文件中的本地头像路径，待找到处理办法后再修复。

> 虽然但是，一个基本的逻辑是，如果期望 `bili-sync` 在 NFO 中写入的头像绝对路径能够被 EMBY 读取到，那么两个容器中头像的绝对路径必须完全相同。因此虽然头像还没办法正常加载，但为后续考虑，还是推荐将 THUMB_PATH 填写上，并确保该路径在 `bili-sync` 和 `emby` 两个容器中指向的是相同的文件夹（也就是把一个文件夹同时挂载到 `bili-sync` 和 `emby` 的 THUMB_PATH 下）。

## Docker 运行示例

docker compose 文件：
```yaml
services:
  bili-sync:
    image: amtoaer/bili-sync:latest
    user: 1000:1000  # 此处可以指定以哪个用户的权限运行，不填写的话默认 root，推荐填写。
    tty: true  # 加上这一行可以让日志变成彩色
    volumes:
      - /home/amtoaer/Videos/Bilibilis/:/Videos/Bilibilis/  # 视频文件
      - /home/amtoaer/.config/nas/bili-sync/config/:/app/config/  # 配置文件
      - /home/amtoaer/.config/nas/bili-sync/data/:/app/data/  # 数据库
    environment:
      - THUMB_PATH=/Videos/Bilibilis/thumb/  # 将头像放到视频文件的 thumb 文件夹下
    restart: always
    network_mode: bridge
    hostname: bili-sync
    container_name: bili-sync
    logging:
      driver: "json-file"
      options:
        max-size: "30m"

```

对应的配置文件：

```json
{
    "sessdata": "xxxxxxxxxxxxxxxxxx",
    "bili_jct": "xxxxxxxxxxxxxxxxxx",
    "buvid3": "xxxxxxxxxxxxxxxxxx",
    "dedeuserid": "xxxxxxxxxxxxxxxxxx",
    "ac_time_value": "xxxxxxxxxxxxxxxxxx",
    "interval": 20,
    "favorite_ids": [
        711322958
    ],
    "path_mapper": {
        "711322958": "/Videos/Bilibilis/Bilibili-711322958/"
    }
}
```

## 目前的问题

- [ ] 研究一下 NFO，看看怎么正常读取本地的演员头像

## 路线图

- [x] 凭证认证
- [x] 视频选优
- [x] 视频下载
- [x] 支持并行下载
- [x] 支持作为 daemon 运行
- [x] 构建 nfo 和 poster 文件，方便以单集形式导入 emby
- [x] 支持收藏夹翻页，下载全部历史视频
- [x] 对接数据库，提前检查，按需下载
- [x] 提供简单易用的打包（如 docker）
