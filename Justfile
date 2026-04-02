clean:
    rm -rf ./bili-sync-rs-Linux*.tar.gz

build-frontend:
    cd ./web && bun run build && cd ..

build platform="auto": build-frontend
    ./scripts/build_local.sh binary release "{{platform}}"

build-debug platform="auto": build-frontend
    ./scripts/build_local.sh binary debug "{{platform}}"

build-docker platform="auto" tag="bili-sync-rs-local": build-frontend
    ./scripts/build_local.sh docker release "{{platform}}" "{{tag}}"

build-docker-debug platform="auto" tag="bili-sync-rs-local": build-frontend
    ./scripts/build_local.sh docker debug "{{platform}}" "{{tag}}"

debug: build-frontend
    cargo run
