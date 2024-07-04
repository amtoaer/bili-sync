---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

title: bili-sync
titleTemplate: 由 Rust & Tokio 驱动的哔哩哔哩同步工具

hero:
  name: "bili-sync"
  text: "由 Rust & Tokio 驱动的哔哩哔哩同步工具"
  # tagline: My great project tagline
  actions:
    - theme: brand
      text: 什么是 bili-sync？
      link: /introduction
    - theme: alt
      text: 快速开始
      link: /quick-start
    - theme: alt
      text: GitHub
      link: https://github.com/amtoaer/bili-sync
  image:
    src: /assets/icon.png
    alt: bili-sync

features:
  - icon: 🤖
    title: 无需干预
    details: 自动选择最优的视频和音频配置
  - icon: 💾
    title: 专为 NAS 设计
    details: 可被 Emby、Jellyfin 等媒体服务器一键识别
  - icon: 🐳
    title: 部署简单
    details: 提供简单易用的 docker 镜像
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #bd34fe 30%, #41d1ff);

  --vp-home-hero-image-background-image: linear-gradient(-45deg, #bd34fe 50%, #47caff 50%);
  --vp-home-hero-image-filter: blur(44px);
}

@media (min-width: 640px) {
  :root {
    --vp-home-hero-image-filter: blur(56px);
  }
}

@media (min-width: 960px) {
  :root {
    --vp-home-hero-image-filter: blur(68px);
  }
}
</style>