run:
    cargo build --bin api --bin telegram
    API_PORT=3007
    cargo run --bin api & cargo run --bin telegram