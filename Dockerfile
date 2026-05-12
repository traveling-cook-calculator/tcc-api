FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --no-create-home --shell /bin/false appuser

COPY tcc_api /usr/local/bin/tcc_api
RUN chmod +x /usr/local/bin/tcc_api

USER appuser
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/tcc_api"]