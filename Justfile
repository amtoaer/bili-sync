clean:
    rm -rf ./bili-sync-rs-Linux*.tar.gz

build-frontend:
    cd ./web && bun run build && cd ..

build: build-frontend
    cargo build --target x86_64-unknown-linux-musl --release

build-debug: build-frontend
    cargo build --target x86_64-unknown-linux-musl

build-docker: build
    tar czvf ./bili-sync-rs-Linux-x86_64-musl.tar.gz -C ./target/x86_64-unknown-linux-musl/release/ ./bili-sync-rs
    docker build . -t bili-sync-rs-local --build-arg="TARGETPLATFORM=linux/amd64"
    just clean

build-docker-debug: build-debug
    tar czvf ./bili-sync-rs-Linux-x86_64-musl.tar.gz -C ./target/x86_64-unknown-linux-musl/debug/ ./bili-sync-rs
    docker build . -t bili-sync-rs-local --build-arg="TARGETPLATFORM=linux/amd64"
    just clean

debug: build-frontend
    cargo run
