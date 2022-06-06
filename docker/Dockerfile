# This is the official Rust image with Debian 11 "bullseye" slim
FROM rust:slim-bullseye

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get clean

# Set locale
RUN apt-get update && apt-get install -y locales && rm -rf /var/lib/apt/lists/* \
	&& localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8
ENV LANG en_US.utf8

# Install dependencies
# This is a dependency to build one of the rust package dependencies
RUN apt-get update && apt-get install -y gcc
RUN apt-get update && apt-get install -y pkg-config
RUN apt-get update && apt-get install -y libssl-dev
# prost dependencies
RUN apt-get install -y cmake
RUN apt-get install -y g++

# For command line gRPC testing
RUN apt-get install -y wget && \
    wget https://github.com/fullstorydev/grpcurl/releases/download/v1.8.6/grpcurl_1.8.6_linux_x86_64.tar.gz && \
    tar -xvf grpcurl_1.8.6_linux_x86_64.tar.gz && \
    chmod +x grpcurl && \
    cp grpcurl /usr/local/bin

RUN rustup component add clippy
RUN rustup component add rustfmt
