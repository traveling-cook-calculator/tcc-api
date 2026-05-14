FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libpq5 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --no-create-home --shell /bin/false tcc

COPY --chmod=755 tcc_api /usr/local/bin/tcc_api

USER tcc
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/tcc_api"]