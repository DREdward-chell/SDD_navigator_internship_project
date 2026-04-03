# Build stage — rust:alpine compiles natively against musl, no cross-compilation needed
FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --workspace --release

# Runtime stage
FROM alpine:latest
WORKDIR /app
COPY --from=builder /build/target/release/sdd-server /app/sdd-server
COPY --from=builder /build/target/release/sdd-coverage /app/sdd-coverage
ENV SDD_PORT=4010
ENV SDD_PROJECT_ROOT=/workspace
EXPOSE 4010
ENTRYPOINT ["/app/sdd-server"]
