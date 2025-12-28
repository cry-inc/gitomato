# Temporary build container
FROM rust:1-alpine AS builder

# Copy source code into container
WORKDIR /usr/src
COPY . .

# Build Rust binary
ENV CARGO_TARGET_DIR=/usr/src/target
RUN cargo build --release

# Remove debug symbols
RUN strip /usr/src/target/release/gitomato

# Build final minimal image with only the binary
FROM scratch
COPY --from=builder /usr/src/target/release/gitomato /
EXPOSE 8080
STOPSIGNAL SIGINT
CMD ["/gitomato"]
