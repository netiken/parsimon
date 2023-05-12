# Use a Rust base image
FROM rust:latest AS builder

# Create a new directory for the application
WORKDIR /usr/src/parsimon-worker

# Copy the source code
COPY . .

# Build the dependencies
RUN rustup override set nightly

# Build the application
RUN cargo build --release --bin parsimon-worker

# RUN apt-get update & apt-get install -y extra-runtime-dependencies & rn -rf /var/lib/apt/lists/*
# Create a new image from the debian base image
FROM debian:bullseye

# Copy the built executable from the builder image
COPY --from=builder /usr/src/parsimon-worker/target/release/parsimon-worker /usr/local/bin/parsimon-worker
CMD ["parsimon-worker"]