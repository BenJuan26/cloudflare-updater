FROM rust:1.81 AS build

# create a new empty shell project
RUN cargo new --bin cloudflare-updater
WORKDIR /cloudflare-updater
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# build deps
RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

# build for release
RUN rm ./target/release/deps/cloudflare_updater*
RUN cargo build --release

FROM debian:trixie-slim

# copy the build artifact from the build stage
COPY --from=build /cloudflare-updater/target/release/cloudflare-updater .

CMD ["./cloudflare-updater"]