default:
    just --list

test:
    cargo test --all

test-ob:
    cargo test -p tonic-sdk-dex-orderbook