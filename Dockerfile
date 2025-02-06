# Build stage using rust:alpine
FROM rust:1.83-alpine3.20 AS builder

WORKDIR /app

RUN apk add --no-cache \
    build-base \
    perl \
    pkgconfig \
    libffi-dev \
    musl-dev \
    musl \
    openssl

RUN apk add --no-cache openssl-dev

RUN apk add --no-cache openssl-libs-static

COPY Cargo.toml Cargo.lock ./

COPY ./src ./src

RUN cargo build --release


FROM alpine:latest

WORKDIR /app

ARG PROD_DB_URL
ARG JWT_SECRET
ARG PROD_RP_ORIGIN
ARG PROD_RP_ID
ARG PROD_CLIENT_ORIGIN
ARG PROD_SERVER_ADDR
ARG TOKEN_SECRET
ARG IS_DEV=false


ENV DEV_DB_URL=${PROD_DB_URL}
ENV JWT_SECRET=${JWT_SECRET}
ENV DEV_RP_ORIGIN=${PROD_RP_ORIGIN}
ENV DEV_RP_ID=${PROD_RP_ID}
ENV DEV_CLIENT_ORIGIN=${PROD_CLIENT_ORIGIN}
ENV DEV_SERVER_ADDR=${PROD_SERVER_ADDR}
ENV TOKEN_SECRET=${TOKEN_SECRET}
ENV IS_DEV=${IS_DEV}


COPY --from=builder /app/target/release/polling-app-backend /usr/local/bin/polling-app-backend

EXPOSE 5000

CMD [ "/usr/local/bin/polling-app-backend" ]