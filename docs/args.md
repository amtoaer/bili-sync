# 命令行参数

程序支持有限的命令行参数，可以通过执行 `bili-sync-rs --help` 查看说明。

```shell
bili-sync/target/debug docs_vitepress* ⇡
❯ ./bili-sync-rs --help
基于 rust tokio 编写的 bilibili 收藏夹同步下载工具

Usage: bili-sync-rs [OPTIONS]

Options:
  -s, --scan-only              [env: SCAN_ONLY=]
  -l, --log-level <LOG_LEVEL>  [env: RUST_LOG=] [default: None,bili_sync=info]
  -h, --help                   Print help
  -V, --version                Print version
```

可以看到除版本和帮助信息外，程序仅支持两个参数，参数除可以通过命令行设置外，还可通过环境变量设置。

## `--scan-only`

`--scan-only` 参数用于仅扫描列表，而不实际执行下载操作。该参数的主要目的是[方便用户从 v1 迁移](https://github.com/amtoaer/bili-sync/issues/66#issuecomment-2066642481)，新用户不需要关注。

## `--log-level`

`--log-level` 参数用于设置日志级别，一般可以维持默认。该参数与 Rust 程序中 `RUST_LOG` 的语义相同，可以查看[相关文档](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)获取详细信息。