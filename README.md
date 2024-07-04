![bili-sync](https://socialify.git.ci/amtoaer/bili-sync/image?description=1&font=KoHo&issues=1&language=1&logo=https%3A%2F%2Fs2.loli.net%2F2023%2F12%2F02%2F9EwT2yInOu1d3zm.png&name=1&owner=1&pattern=Signal&pulls=1&stargazers=1&theme=Light)

## 简介

> [!NOTE]
> 此为 v2.x 版本文档，v1.x 版本文档请前往[此处](https://github.com/amtoaer/bili-sync/tree/v1.x)查看。


为 NAS 用户编写的 BILIBILI 收藏夹同步工具，可使用 EMBY 等媒体库工具浏览。

支持展示视频封面、名称、加入日期、标签、分页等。


## 效果演示

**注：因为可能同时存在单页视频和多页视频，媒体库类型请选择“混合内容”。**

### 概览
![概览](./assets/overview.png)
### 详情
![详情](./assets/detail.png)
### 播放（使用 infuse）
![播放](./assets/play.png)
### 文件排布
![文件](./assets/dir.png)


## 功能与路线图

- [x] 使用用户填写的凭据认证，并在必要时自动刷新
- [x] 支持收藏夹与视频列表/视频合集的下载
- [x] 自动选择用户设置范围内最优的视频和音频流，并在下载完成后使用 FFmpeg 合并
- [x] 使用 Tokio 与 Reqwest，对视频、视频分页进行异步并发下载
- [x] 使用媒体服务器支持的文件命名，方便一键作为媒体库导入
- [x] 当前轮次下载失败会在下一轮下载时重试，失败次数过多自动丢弃
- [x] 使用数据库保存媒体信息，避免对同个视频的多次请求
- [x] 打印日志，并在请求出现风控时自动终止，等待下一轮执行
- [x] 提供多平台的二进制可执行文件，为 Linux 平台提供了立即可用的 Docker 镜像
- [ ] 支持对“稍后再看”内视频的自动扫描与下载
- [ ] 下载单个文件时支持断点续传与并发下载


## 参考与借鉴

该项目实现过程中主要参考借鉴了如下的项目，感谢他们的贡献：

+ [bilibili-API-collect](https://github.com/SocialSisterYi/bilibili-API-collect) B 站的第三方接口文档
+ [bilibili-api](https://github.com/Nemo2011/bilibili-api) 使用 Python 调用接口的参考实现
+ [danmu2ass](https://github.com/gwy15/danmu2ass) 本项目弹幕下载功能的缝合来源
