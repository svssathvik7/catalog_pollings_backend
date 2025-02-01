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


FROM alpine:latest AS runner

WORKDIR /app

# Group non-sensitive environment variables
ENV IS_DEV=true \
    DEV_RP_ID=localhost \
    DEV_RP_ORIGIN=http://localhost:3000 \
    DEV_CLIENT_ORIGIN=http://localhost:3000 \
    DEV_SERVER_ADDR=localhost \
    RUST_LOG=trace

# Use build arguments for sensitive data, but don't set defaults
ARG TOKEN_SECRET
ARG JWT_SECRET
ARG PROD_DB_URL
ARG PROD_RP_ID
ARG PROD_RP_ORIGIN
ARG PROD_CLIENT_ORIGIN
ARG PROD_SERVER_ADDR
ARG IS_DEV=true

# Set sensitive environment variables at runtime
ENV IS_DEV=${IS_DEV}
ENV TOKEN_SECRET=${TOKEN_SECRET}
ENV JWT_SECRET=${JWT_SECRET}
ENV DEV_DB_URL=${DEV_DB_URL}

COPY --from=builder /app/target/release/polling-app-backend /usr/local/bin/polling-app-backend

EXPOSE 3001

CMD [ "/usr/local/bin/polling-app-backend" ]