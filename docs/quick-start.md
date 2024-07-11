# 快速开始

程序使用 Rust 编写，不需要 Runtime 且并为各个平台提供了预编译文件，绝大多数情况下是没有使用障碍的。

## 程序获取

程序为各个平台提供了预构建的二进制文件，并且打包了 `Linux/amd64` 与 `Linux/arm64` 两个平台的 Docker 镜像。用户可以自行选择使用哪种方式运行。

### 其一：下载平台二进制文件运行

> [!CAUTION]
> 如果你使用这种方式运行，请确保 FFmpeg 已被正确安装且位于 PATH 中，可通过执行 `ffmpeg` 命令访问。

在[程序发布页](https://github.com/amtoaer/bili-sync/releases)选择最新版本中对应机器架构的压缩包，解压后会获取一个名为 `bili-sync-rs` 的可执行文件，直接双击执行。

### 其二： 使用 Docker Compose 运行

Linux/amd64 与 Linux/arm64 两个平台可直接使用 Docker 或 Docker Compose 运行，此处以 Compose 为例：
> 请注意其中的注释，有不清楚的地方可以先继续往下看。

```yaml
services:
  bili-sync-rs:
    # 不推荐使用 latest 这种模糊的 tag，最好直接指明版本号
    image: amtoaer/bili-sync-rs:latest
    restart: unless-stopped
    network_mode: bridge
    # 该选项请仅在日志终端支持彩色输出时启用，否则日志中可能会出现乱码
    tty: true
    # 非必需设置项，推荐设置为宿主机用户的 uid 及 gid (`$uid:$gid`)
    # 可以执行 `id ${user}` 获取 `user` 用户的 uid 及 gid
    # 程序下载的所有文件权限将与此处的用户保持一致，不设置默认为 Root
    user: 1000:1000
    hostname: bili-sync-rs
    container_name: bili-sync-rs
    volumes:
      - ${你希望存储程序配置的目录}:/app/.config/bili-sync
      # 还需要有一些其它必要的挂载，包括 up 主信息位置、视频下载位置
      # 这些目录不是固定的，只需要确保此处的挂载与 bili-sync-rs 的配置文件相匹配
      # ...
    # 如果你使用的是群晖系统，请移除最后的 logging 配置，否则会导致日志不显示
    logging:
      driver: "local"
```

使用该 compose 文件，执行 `docker compose up -d` 即可运行。

## 程序配置

> [!NOTE]
> 在 Docker 环境中，`~` 会被展开为 `/app`。

你是否遇到了程序的 panic？别担心，这是正常情况。

程序默认会将配置文件存储于 `~/.config/bili-sync/config.toml`，数据库文件存储于 `~/.config/bili-sync/data.sqlite`。

在启动时程序会尝试加载配置文件，如果发现不存在会新建并写入默认配置。

获得配置内容后，程序会对其做一次简单的校验，因为默认配置中不包含凭据信息与要下载的收藏夹、视频合集/视频列表，因此程序会拒绝运行而发生 panic。我们只需要在程序生成的默认配置上做一些简单修改即可成功运行。

当前版本的默认示例文件如下：
```toml
video_name = "{{title}}"
page_name = "{{bvid}}"
interval = 1200
upper_path = "/Users/amtoaer/Library/Application Support/bili-sync/upper_face"
nfo_time_type = "favtime"

[credential]
sessdata = ""
bili_jct = ""
buvid3 = ""
dedeuserid = ""
ac_time_value = ""

[filter_option]
video_max_quality = "Quality8k"
video_min_quality = "Quality360p"
audio_max_quality = "QualityHiRES"
audio_min_quality = "Quality64k"
codecs = [
    "AV1",
    "HEV",
    "AVC",
]
no_dolby_video = false
no_dolby_audio = false
no_hdr = false
no_hires = false

[danmaku_option]
duration = 15.0
font = "黑体"
font_size = 25
width_ratio = 1.2
horizontal_gap = 20.0
lane_size = 32
float_percentage = 0.5
bottom_percentage = 0.3
opacity = 76
bold = true
outline = 0.8
time_offset = 0.0

[favorite_list]

[collection_list]

[watch_later]
enabled = false
path = ""
```

看起来很长，但绝大部分选项是不需要做修改的。正常情况下，我们只需要关注：
+ `interval`
+ `upper_path`
+ `credential`
+ `codecs`
+ `favorite_list`
+ `collection_list`

以下逐条说明。

### `interval`

表示程序每次执行扫描下载的间隔时间，单位为秒。

### `upper_path`

UP 主头像和信息的保存位置。对于使用 Emby、Jellyfin 媒体服务器的用户，需确保此处路径指向 Emby、Jellyfin 配置中的 `/metadata/people/` 才能够正常在媒体服务器中显示 UP 主的头像。

### `credential`

哔哩哔哩账号的身份凭据，请参考[凭据获取流程](https://nemo2011.github.io/bilibili-api/#/get-credential)获取并对应填写至配置文件中，后续 bili-sync 会在必要时自动刷新身份凭据，不再需要手动管理。

推荐使用匿名窗口获取，避免潜在的冲突。

### `codecs`

这是 bili-sync 选择视频编码的优先级顺序，优先级按顺序从高到低。此处对编码格式做一个简单说明：

+ AVC 又称 H.264，是目前使用最广泛的视频编码格式，绝大部分设备可以使用硬件解码播放该格式的视频（也因此播放普遍流畅），但是同等画质下视频体积较大。

+ HEV(C) 又称 H.265，与 AV1 都是新一代的视频编码格式。这两种编码相比 AVC 有更好的压缩率，同等画质下视频体积更小，但由于相对较新，硬件解码支持不如 AVC 广泛。如果你的播放设备不支持则只能使用软件解码播放，这种情况下可能导致播放卡顿、机器发热等问题。

建议查阅自己常用播放设备对这三种编码的硬件解码支持情况以选择合适的编码格式，如果硬件支持 HEV 或 AV1，那么可以将其优先级调高。

而如果你的设备不支持，或者单纯懒得查询，那么推荐将 AVC 放在第一位以获得最好的兼容性。

### `favorite_list`

你想要下载的收藏夹与想要保存的位置。简单示例：
```toml
3115878158 = "/home/amtoaer/Downloads/bili-sync/测试收藏夹"
```
收藏夹 ID 的获取方式可以参考[这里](/favorite)。

### `collection_list`

你想要下载的视频合集/视频列表与想要保存的位置。注意“视频合集”与“视频列表”是两种不同的类型。在配置文件中需要做区分：
```toml
"series:387051756:432248" = "/home/amtoaer/Downloads/bili-sync/测试视频列表"
"season:1728547:101343" = "/home/amtoaer/Downloads/bili-sync/测试合集"
```

具体说明可以参考[这里](/collection)。

### `watch_later`

设置稍后再看的扫描开关与保存位置。

如果你希望下载稍后再看列表中的视频，可以将 `enabled` 设置为 `true`，并填写 `path`。

```toml
enabled = true
path = "/home/amtoaer/Downloads/bili-sync/稍后再看"
```

## 运行

在配置文件填写完毕后，我们可以直接运行程序。如果配置文件无误，程序会自动开始下载收藏夹中的视频。并每隔 `interval` 秒重新扫描一次。

如果你希望了解更详细的配置项说明，可以查询[这里](/configuration)。
