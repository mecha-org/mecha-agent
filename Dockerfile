#######################
#
# BUILDER
#
#######################
FROM rust:1.72-bookworm as builder
WORKDIR /usr/app

# RUN apk add --no-cache musl-dev openssl openssl-dev protoc protobuf protobuf-dev libcrypto3 libssl3
RUN apt update && \
    apt install -y openssl libssl-dev protobuf-compiler libprotoc-dev


COPY ./ ./
RUN cargo build --release

#######################
#
# RUNNER
#
#######################
FROM debian:stable-slim as runner
WORKDIR /usr/app
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

EXPOSE 3001

COPY --from=builder /usr/app/target/release/mecha_agent_server ./
COPY ./settings.yml ./

CMD ["/usr/app/mecha_agent_server", "-s", "/usr/app/settings.yml"]
