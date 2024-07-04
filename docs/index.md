---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "bili-sync"
  text: "基于 rust tokio 的 bilibili 同步工具"
  # tagline: My great project tagline
  actions:
    - theme: brand
      text: 快速开始
      link: /contents/quick-start
    - theme: alt
      text: 介绍
      link: /contents/intro
    - theme: alt
      text: GitHub
      link: https://github.com/amtoaer/bili-sync
  image:
    src: /assets/icon.png
    alt: bili-sync

features:
  - icon: 💾
    title: 无需干预
    details: 自动选择最优的视频和音频配置
  - icon: ☁️
    title: 专为 NAS 设计
    details: 使用可被 Emby、Jellyfin 等工具一键识别的命名方式
  - icon: 🐳
    title: 部署简单
    details: 提供简单易用的 docker 镜像
---

