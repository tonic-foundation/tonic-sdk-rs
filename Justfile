default:
    just --list

fuzz-ob:
    cargo test -p tonic-sdk-dex-orderbook --features tonic-sdk-dex-orderbook/fuzz
