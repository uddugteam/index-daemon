FROM debian:bullseye-slim
WORKDIR /app
ADD target/release/index-daemon .
RUN apt-get update \
        && apt-get -y install make openssh-client ca-certificates && update-ca-certificates
CMD ["/app/index-daemon"]
