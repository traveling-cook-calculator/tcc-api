FROM alpine:3 AS base
RUN apk add --no-cache ca-certificates

FROM scratch

COPY --from=base /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --chmod=755 tcc_api /tcc_api

USER 10001:10001

EXPOSE 3000
ENTRYPOINT ["/tcc_api"]