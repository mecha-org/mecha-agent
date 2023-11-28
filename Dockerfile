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

# Set the version and URL for the OpenTelemetry Collector contrib binary
ENV OTLP_COL_VERSION=0.86.0
ENV OTLP_COL_PACKAGE_URL=https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/v${OTLP_COL_VERSION}/otelcol-contrib_0.86.0_linux_amd64.tar.gz

# Create parent directory
RUN mkdir -p /etc/mecha && \
    mkdir /etc/mecha/otel-collector

# Copy your collector configuration file into the container
COPY ./otel-contrib.yml /etc/mecha/otel-contrib.yml

# Install necessary tools
RUN apt-get update && \
    apt-get install -y wget tar

# Download and extract the OpenTelemetry Collector contrib binary
RUN wget -O /tmp/otelcol-contrib.tar.gz $OTLP_COL_PACKAGE_URL && \
    tar -xzf /tmp/otelcol-contrib.tar.gz -C /tmp && \
    mv /tmp/otelcol-contrib /etc/mecha/otelcol-contrib

# Cleanup
RUN rm /tmp/otelcol-contrib.tar.gz

EXPOSE 3001

COPY --from=builder /usr/app/target/release/mecha_agent_server ./
COPY ./settings.yml ./

CMD ["/usr/app/mecha_agent_server", "-s", "/usr/app/settings.yml"]
