# EduCore — Northflank / istalgan Docker hosting uchun
FROM rust:1-bookworm AS build
WORKDIR /app
COPY rust-toolchain Cargo.toml Cargo.lock ./
COPY src ./src
COPY web ./web
COPY migrations ./migrations
COPY tests ./tests
RUN cargo build --release --locked -j 2

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/educore /app/educore
ENV PORT=8080
EXPOSE 8080
CMD ["/app/educore"]
