default:
    just --list

fuzz:
    cargo test -p tonic-sdk-dex-orderbook --features fuzz
