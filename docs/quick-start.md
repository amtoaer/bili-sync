# 快速开始

程序使用 Rust 编写，不需要 Runtime 且内嵌 WebUI，并为各个平台提供了预编译的二进制文件，因此部署较为简单。

## 程序获取

程序为各个平台提供了预构建的二进制文件，并且打包了 `Linux/amd64` 与 `Linux/arm64` 两个平台的 Docker 镜像。用户可以自行选择使用哪种方式运行。

### 其一：下载平台二进制文件运行

> [!CAUTION]
> 如果你使用这种方式运行，请确保 FFmpeg 已被正确安装且位于 PATH 中，可直接通过 `ffmpeg` 命令访问。

在[程序发布页](https://github.com/amtoaer/bili-sync/releases)选择最新版本中对应机器架构的压缩包，解压后会获取一个名为 `bili-sync-rs` 的可执行文件，直接双击执行。

### 其二：使用 Docker Compose 运行

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
    # 程序默认绑定 0.0.0.0:12345 运行 http 服务
    # 可同时修改 compose 文件与 config.toml 变更服务运行的端口
    ports:
      - 12345:12345
    volumes:
      - ${你希望存储程序配置的目录}:/app/.config/bili-sync
      # metadata/people 正确挂载才能在 Emby 或 Jellyfin 中显示 UP 主头像
      # 右边的目标目录不固定，只需要确保目标目录与 bili-sync 中填写的“UP 主头像保存路径”保持一致即可
      - ${Emby 或 Jellyfin 配置下的 metadata/people 目录}:/app/.config/bili-sync/upper_face
      # 接下来可以挂载一系列用于保存视频的目录，接着在 bili-sync 中配置将视频下载到这些目录即可
      # 例如：
      # - /home/amtoaer/HDDs/Videos/Bilibilis/:/home/amtoaer/HDDs/Videos/Bilibilis/
    # 如果你使用的是群晖系统，请移除最后的 logging 配置，否则会导致日志不显示
    logging:
      driver: "local"
```

使用该 compose 文件，执行 `docker compose up -d` 即可运行。

## 进行必要配置

运行程序，应该可以在日志中看到：
```
Jul 12 16:11:10  INFO 欢迎使用 Bili-Sync，当前程序版本：xxxxx
Jul 12 16:11:10  INFO 项目地址：https://github.com/amtoaer/bili-sync
Jul 12 16:11:10  INFO 数据库初始化完成
Jul 12 17:17:50  WARN 生成 auth_token：xxxxxxxx，可使用该 token 登录 web UI，该信息仅在首次运行时打印
Jul 12 16:11:10  INFO 配置初始化完成
Jul 12 16:11:10  INFO 开始运行管理页: http://0.0.0.0:12345
```

中间应该会穿插一条 CONFIG 的报错，这是因为配置文件内容缺失导致视频下载任务未能运行，在初次启动时是正常现象。

自 2.6.0 版本开始，程序仅会创建一个数据库文件，配置同样在数据库表中进行维护。

数据库文件存储于 `${config_dir}/bili-sync/data.sqlite`。

> [!CAUTION]
>
> 请注意，`config_dir` 的实际位置与操作系统和用户名有关。
>
> 对于名为 Alice 的用户，`config_dir` 指向的位置是：
>
> + Lin: `/home/Alice/.config`
> + Win: `C:\Users\Alice\AppData\Roaming`
> + Mac: `/Users/Alice/Library/Application Support`
>
> 特别的，在 Docker 环境中，`config_dir` 会被展开为 `/app/.config`。

接着打开 WebUI，切换到设置页，输入日志中打印的 `auth_token`，点击认证。

![设置页](/assets/config.webp)

认证后会看到一系列的配置，除绑定地址外的选项**基本都会实时生效**。为避免意料外的情况，建议将配置文件一次修改完毕后再点击保存。

如无特殊需求，一般仅需修改“B 站认证”与“视频质量”两个标签下的配置。

其中“B 站认证”在一次填写后即可忽略，程序会在**每日第一次运行视频下载任务**时检查认证状态，并在有必要时自动刷新。

对于这些设置项的含义，请参考[配置说明](./configuration.md)，可善用右侧导航在不同配置项间跳转。

## 添加视频源订阅

配置完毕后，我们便可以随时添加视频源订阅。

用户在正确填写“B 站认证”后可以在“快捷订阅”部分查看自己创建的收藏夹、关注的合集与 UP 主一键订阅，也可以在“视频源”页手动添加并管理。

对于手动添加的视频源，可参考如下页面获取所需的参数：

- [收藏夹](./favorite.md)
- [合集 / 列表](./collection.md)
- [用户投稿](./submission.md)

添加完订阅就无需进行任何干预了，视频下载任务会在后台每隔特定时间（由配置中的“同步间隔”决定）自动运行一次，刷新并下载启用的视频源！