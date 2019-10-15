FROM ubuntu:latest as base

RUN apt-get update && apt-get -y install ca-certificates libssl-dev

RUN mkdir -p /app

FROM base as build

RUN apt-get -y install curl build-essential pkg-config

COPY ./rust-toolchain /app

# Install Rust environment
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $(cat /app/rust-toolchain)
ENV PATH="/root/.cargo/bin:${PATH}"

# Buikd
RUN mkdir -p /app
COPY ./ /app
WORKDIR /app
RUN cargo build --release 

FROM base as release

COPY --from=build /app/target/release/yells-at /app

EXPOSE 3000

CMD /app/yells-at
