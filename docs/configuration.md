# 配置文件

默认的配置文件已经在[快速开始](/quick-start)中给出，该文档对配置文件的各个参数依次详细解释。

## video_name 与 page_name

`video_name` 与 `page_name` 用于设置下载文件的命名规则，对于所有下载的内容，将会维持如下的目录结构：

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

这两个参数支持使用模板，其中用 <code v-pre>{{  }}</code> 包裹的模板变量在执行时会被动态替换为对应的内容。

对于 `video_name`，支持设置 bvid（视频编号）、title（视频标题）、upper_name（up 主名称）、upper_mid（up 主 id）、pubtime（视频发布时间）、fav_time（视频收藏时间）。

对于 `page_name`，除支持 video 的全部参数外，还支持 ptitle（分 P 标题）、pid（分 P 页号）。

为了解决文件名可能过长的问题，程序为模板引入了 `truncate` 函数。如 <code v-pre>{{ truncate title 10 }}</code> 表示截取 `title` 的前 10 个字符。

> [!TIP]
> 1. 仅收藏夹视频会区分 `fav_time` 和 `pubtime`，其它类型下载两者的取值是完全相同的；
> 2. `fav_time` 和 `pubtime` 的格式受 `time_format` 参数控制，详情可参考 [time_format 小节](#time-format)。

此外，`video_name` 和 `page_name` 还支持使用路径分割符，如 <code v-pre>{{ upper_mid }}/{{ title }}_{{ pubtime }}</code> 表示视频会根据 UP 主 id 将视频分到不同的文件夹中。

推荐仅在 `video_name` 中使用路径分割符，暂不清楚在 `page_name` 中使用路径分割符导致分页存储到子文件夹后是否还能被媒体服务器正确识别。

> [!CAUTION]
> **路径分隔符**在不同平台定义不同，Windows 下为 `\`，MacOS 和 Linux 下为 `/`。

## `interval`

表示程序每次执行扫描下载的间隔时间，单位为秒。

## `upper_path`

UP 主头像和信息的保存位置。对于使用 Emby、Jellyfin 媒体服务器的用户，需确保此处路径指向 Emby、Jellyfin 配置中的 `/metadata/people/` 才能够正常在媒体服务器中显示 UP 主的头像。

## `nfo_time_type`

表示在视频信息中使用的时间类型，可选值为 `favtime`（收藏时间）和 `pubtime`（发布时间）。

仅收藏夹视频会区分 `fav_time` 和 `pubtime`，其它类型下载两者取值相同。

## `time_format`

时间格式，用于设置 `fav_time` 和 `pubtime` 在 `video_name`、 `page_name` 中使用时的显示格式，支持的格式符号可以参考 [chrono strftime 文档](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)。

## `credential`

哔哩哔哩账号的身份凭据，请参考[凭据获取流程](https://nemo2011.github.io/bilibili-api/#/get-credential)获取并对应填写至配置文件中，后续 bili-sync 会在必要时自动刷新身份凭据，不再需要手动管理。

推荐使用匿名窗口获取，避免潜在的冲突。

## `filter_option`

过滤选项，用于设置程序的过滤规则，程序会从过滤结果中选择最优的视频、音频流下载。

这些内容的可选值可前往 [analyzer.rs](https://github.com/amtoaer/bili-sync/blob/24d0da0bf3ea65fd45d07587e4dcdbb24d11a589/crates/bili_sync/src/bilibili/analyzer.rs#L10-L55) 中查看。

注意将过滤范围设置过小可能导致筛选不到符合要求的流导致下载失败，建议谨慎修改。

### `video_max_quality`

视频流允许的最高质量。

### `video_min_quality`

视频流允许的最低质量。

### `audio_max_quality`

音频流允许的最高质量。

### `audio_min_quality`

音频流允许的最低质量。

### `codecs`

这是 bili-sync 选择视频编码的优先级顺序，优先级按顺序从高到低。此处对编码格式做一个简单说明：

+ AVC 又称 H.264，是目前使用最广泛的视频编码格式，绝大部分设备可以使用硬件解码播放该格式的视频（也因此播放普遍流畅），但是同等画质下视频体积较大。

+ HEV(C) 又称 H.265，与 AV1 都是新一代的视频编码格式。这两种编码相比 AVC 有更好的压缩率，同等画质下视频体积更小，但由于相对较新，硬件解码支持不如 AVC 广泛。如果你的播放设备不支持则只能使用软件解码播放，这种情况下可能导致播放卡顿、机器发热等问题。

建议查阅自己常用播放设备对这三种编码的硬件解码支持情况以选择合适的编码格式，如果硬件支持 HEV 或 AV1，那么可以将其优先级调高。

而如果你的设备不支持，或者单纯懒得查询，那么推荐将 AVC 放在第一位以获得最好的兼容性。

### `no_dolby_video`

是否禁用杜比视频流。

### `no_dolby_audio`

是否禁用杜比音频流。

### `no_hdr`

是否禁用 HDR 视频流。

### `no_hires`

是否禁用 Hi-Res 音频流。

## `danmaku_option`

弹幕的设置选项，用于设置下载弹幕的样式，几乎全部取自[上游仓库](https://github.com/gwy15/danmu2ass)。

### `duration`

弹幕在屏幕上的持续时间，单位为秒。

### `font`

弹幕的字体。

### `font_size`

弹幕的字体大小。

### `width_ratio`

计算弹幕宽度的比例，为避免重叠可以调大这个数值。

### `horizontal_gap`

两条弹幕之间最小的水平距离。

### `lane_size`

弹幕所占据的高度，即“行高度/行间距”。

### `float_percentage`

屏幕上滚动弹幕最多高度百分比。

### `bottom_percentage`

屏幕上底部弹幕最多高度百分比。

### `opacity`

透明度，取值范围为 0-255。透明度可以通过 opacity / 255 计算得到。

### `bold`

是否加粗。

### `outline`

描边宽度。

### `time_offset`

时间轴偏移，>0 会让弹幕延后，<0 会让弹幕提前，单位为秒。

## `favorite_list`

你想要下载的收藏夹与想要保存的位置。简单示例：
```toml
3115878158 = "/home/amtoaer/Downloads/bili-sync/测试收藏夹"
```
收藏夹 ID 的获取方式可以参考[这里](/favorite)。

## `collection_list`

你想要下载的视频合集/视频列表与想要保存的位置。注意“视频合集”与“视频列表”是两种不同的类型。在配置文件中需要做区分：
```toml
"series:387051756:432248" = "/home/amtoaer/Downloads/bili-sync/测试视频列表"
"season:1728547:101343" = "/home/amtoaer/Downloads/bili-sync/测试合集"
```

具体说明可以参考[这里](/collection)。

## `submission_list`

你想要下载的 UP 主投稿与想要保存的位置。简单示例：
```toml
9183758 = "/home/amtoaer/Downloads/bili-sync/测试投稿"
```
UP 主 ID 的获取方式可以参考[这里](/submission)。

## `watch_later`

设置稍后再看的扫描开关与保存位置。

如果你希望下载稍后再看列表中的视频，可以将 `enabled` 设置为 `true`，并填写 `path`。

```toml
enabled = true
path = "/home/amtoaer/Downloads/bili-sync/稍后再看"
```

## `concurrent_limit`

对 bili-sync 的并发下载进行多方面的限制，避免 api 请求过于频繁导致的风控。其中 video 和 page 表示下载任务的并发数，rate_limit 表示 api 请求的流量限制。默认取值为：
```toml
[concurrent_limit]
video = 3
page = 2

[concurrent_limit.rate_limit]
limit = 4
duration = 250
```

具体来说，程序的处理逻辑是严格从上到下的，即程序会首先并发处理多个 video，每个 video 内再并发处理多个 page，程序的并行度可以简单衡量为 `video * page`（很多 video 都只有单个 page，实际会更接近 `video * 1`），配置项中的 `video` 和 `page` 两个参数就是控制此处的，调节这两个参数可以宏观上控制程序的并行度。

另一方面，每个执行的任务内部都会发起若干 api 请求以获取信息，这些请求的整体频率受到 `rate_limit` 的限制，使用漏桶算法实现。如默认配置表示的是每 250ms 允许 4 个 api 请求，超过这个频率的请求会被暂时阻塞，直到漏桶中有空间为止。调节 `rate_limit` 可以从微观上控制程序的并行度，同时也是最直接、最显著的控制 api 请求频率的方法。

据观察 b 站风控限制大多集中在主站，因此目前 `rate_limit` 仅作用于主站的各类请求，如请求各类视频列表、视频信息、获取流下载地址等，对实际的视频、图片下载过程不做限制。

> [!TIP]
> 1. 一般来说，`video` 和 `page` 的值不需要过大；
> 2. `rate_limit` 的值可以根据网络环境和 api 请求频率进行调整，如果经常遇到风控可以优先调小 limit。