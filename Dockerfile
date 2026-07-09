FROM gcr.io/distroless/static-debian12

COPY tcc_api /usr/local/bin/tcc_api

USER nonroot:nonroot

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/tcc_api"]