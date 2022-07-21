FROM debian:bullseye-slim as builder
LABEL description="This is the build stage for Index-daemon. Here we create the binary."

ARG PROFILE=release
WORKDIR /app

ADD . .

RUN apt-get update --fix-missing && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y apt-transport-https cmake pkg-config libssl-dev git clang curl && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	cargo build "--$PROFILE"

# ===== SECOND STAGE ======

FROM debian:bullseye-slim
LABEL description="This is the 2nd stage: a very small image where we copy the Index-daemon binary."
ARG PROFILE=release

COPY --from=builder /app/target/$PROFILE/index-daemon /usr/local/bin

RUN apt-get update --fix-missing && \
    apt-get -y install apt-transport-https make openssh-client ca-certificates heaptrack && \
    update-ca-certificates && \
    mv /usr/share/ca* /tmp && \
    rm -rf /usr/share/*  && \
    mv /tmp/ca-certificates /usr/share/ && \
    ldd /usr/local/bin/index-daemon && \
    /usr/local/bin/index-daemon --version && \
    rm -rf /usr/share/man && \
    rm -rf /var/lib/apt/lists/*
    #rm -rf /usr/lib/python* && \
    #rm -rf /usr/bin /usr/sbin /usr/share/man

CMD ["/usr/local/bin/index-daemon"]
