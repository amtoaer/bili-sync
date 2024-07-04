> [!NOTE]
> 此为 v2.x 版本文档，v1.x 版本文档请前往[此处](https://github.com/amtoaer/bili-sync/tree/v1.x)查看。

bili-sync 是专为 NAS 用户编写的哔哩哔哩同步工具，下载内容可直接导入 EMBY 等媒体库工具浏览。

## 当前功能

- [x] 凭证认证
- [x] 视频选优
- [x] 视频下载
- [x] 支持并发下载
- [x] 支持作为 daemon 运行
- [x] 构建 nfo 和 poster 文件，方便以单集形式导入 emby
- [x] 支持收藏夹翻页，下载全部历史视频
- [x] 对接数据库，提前检查，按需下载
- [x] 支持弹幕下载
- [x] 更好的错误处理
- [x] 更好的日志
- [x] 请求过快出现风控的 workaround
- [x] 提供简单易用的打包（如 docker）
- [x] 支持 UP 主合集下载

## 展示

**注：因为可能同时存在单页视频和多页视频，媒体库类型请选择“混合内容”。**

### 概览
![概览](/assets/overview.png)
### 详情
![详情](/assets/detail.png)
### 播放（使用 infuse）
![播放](/assets/play.png)
### 文件排布
![文件](/assets/dir.png)