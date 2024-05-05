clean:
    rm -rf ./*-bili-sync-rs

build:
    cargo build --target x86_64-unknown-linux-musl --release

build-docker: build
    cp target/x86_64-unknown-linux-musl/release/bili-sync-rs ./Linux-x86_64-bili-sync-rs
    docker build . -t bili-sync-rs-local --build-arg="TARGETPLATFORM=linux/amd64"
    just clean

copy-config:
    rm -rf ~/.config/bili-sync
    cp -r ~/.config/nas/bili-sync-rs ~/.config/bili-sync
    sed -i -e 's/\/Bilibilis/\/Test_Bilibilis/g' -e 's/.config\/nas/.config\/test_nas/g' ~/.config/bili-sync/config.toml

run:
    cargo run

debug: copy-config
    just run