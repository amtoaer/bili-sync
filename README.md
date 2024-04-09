![bili-sync](https://socialify.git.ci/amtoaer/bili-sync/image?description=1&font=KoHo&issues=1&language=1&logo=https%3A%2F%2Fs2.loli.net%2F2023%2F12%2F02%2F9EwT2yInOu1d3zm.png&name=1&owner=1&pattern=Signal&pulls=1&stargazers=1&theme=Light)

## 简介

> [!NOTE]
> 新版本已使用 Rust 重构，该文档是对新版本的说明。对于 v1.x 的 Python 版本，请前往 [v1.x](https://github.com/amtoaer/bili-sync/tree/v1.x) 分支查看。
>
> 目前新版本尚未进行 docker 打包，docker 版本相关问题请同样参考 [v1.x](https://github.com/amtoaer/bili-sync/tree/v1.x) 分支的 README 与 [v1.x 的 release 文档](https://github.com/amtoaer/bili-sync/releases)。

> [!CAUTION]
> 当前新版本尚不稳定，可能会有未告知的不兼容更改，请优先使用 v1.x 的 Python 版本。

为 NAS 用户编写的 BILIBILI 收藏夹同步工具，可使用 EMBY 等媒体库工具浏览。

支持展示视频封面、名称、加入日期、标签、分页等。

## 效果演示

![概览](./assets/overview.png)

![详情](./assets/detail.png)

## 配置文件

程序默认会将配置文件存储于 `~/.config/bili-sync/config.toml`，数据库文件存储于 `~/.config/bili-sync/data.sqlite`，如果发现不存在则新建并写入初始配置。

配置文件加载时会进行简单校验，对于默认的空配置，校验将会报错，程序将会终止运行。

对于配置文件中的 `credential`，请参考[凭据获取流程](https://nemo2011.github.io/bilibili-api/#/get-credential)。

配置文件中的 `video_name` 和 `page_name` 支持使用模板，在执行时会被动态替换为对应的内容。

video_name 支持设置 bvid（视频编号）、title（视频标题）、upper_name（up 主名称）、upper_mid（up 主 id）。

page_name 除支持 video 的全部参数外，还支持 ptitle（分 P 标题）、pid（分 P 页号）。

对于每个 favorite_list 的下载路径，程序会在其下建立如下的文件夹：

1. 单页视频：

    ```bash
    ├── {video_name}
    │   ├── {page_name}.mp4
    │   ├── {page_name}.nfo
    │   └── {page_name}-poster.jpg
    ```

2. 多页视频：

    ```bash
    ├── {video_name}
    │   ├── poster.jpg
    │   ├── Season 1
    │   │   ├── {page_name} - S01E01.mp4
    │   │   ├── {page_name} - S01E01.nfo
    │   │   ├── {page_name} - S01E01-thumb.jpg
    │   │   ├── {page_name} - S01E02.mp4
    │   │   ├── {page_name} - S01E02.nfo
    │   │   └── {page_name} - S01E02-thumb.jpg
    │   └── tvshow.nfo
    ```

对于 filter_option 的可选值，请前往 [analyzer.rs](https://github.com/amtoaer/bili-sync/blob/main/src/bilibili/analyzer.rs) 查看。

## 配置文件示例与说明

```toml
video_name = "{{title}}"
page_name = "{{bvid}}"
interval = 1200

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

[favorite_list]
52642258 = "/home/amtoaer/HDDs/Videos/Bilibilis/混剪"
```

## 路线图

- [x] 凭证认证
- [x] 视频选优
- [x] 视频下载
- [x] 支持并发下载
- [x] 支持作为 daemon 运行
- [x] 构建 nfo 和 poster 文件，方便以单集形式导入 emby
- [x] 支持收藏夹翻页，下载全部历史视频
- [x] 对接数据库，提前检查，按需下载
- [x] 支持弹幕下载
- [ ] 支持视频封面自动裁剪
- [ ] 更好的错误处理
- [ ] 更好的日志
- [ ] 请求过快出现风控的 workaround
- [ ] 提供简单易用的打包（如 docker）
- [ ] 支持 UP 主合集下载
