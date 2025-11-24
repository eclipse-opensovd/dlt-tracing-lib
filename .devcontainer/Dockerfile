# Copyright (c) 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    git \
    cmake \
    build-essential \
    ca-certificates \
    curl \
    zlib1g-dev \
    llvm-dev \
    libclang-dev \
    clang \
    coreutils \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash dlt
RUN mkdir -p /run/dlt

WORKDIR /tmp/dlt-build
RUN git clone https://github.com/COVESA/dlt-daemon.git
WORKDIR /tmp/dlt-build/dlt-daemon

RUN mkdir build && cd build && \
    cmake .. \
    -DDLT_IPC=UNIX_SOCKET \
    -DWITH_DLT_CONSOLE=ON \
    -DWITH_DLT_USE_IPv6=OFF \
    -DDLT_USER_IPC_PATH="/run/dlt" \
    -DDLT_USER=root \
    -DCMAKE_INSTALL_SYSCONFDIR=/etc && \
    make -j$(nproc) && \
    make install && \
    echo "/usr/local/lib" > /etc/ld.so.conf.d/libdlt.conf && \
    ldconfig

# install rust toolchain
RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain 1.88.0 -y

ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /
RUN rm -rf /tmp/dlt-build

# Create workspace directory for devcontainer
RUN mkdir -p /workspace
WORKDIR /workspace

CMD ["/bin/bash"]
