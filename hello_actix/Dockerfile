# Build stage
FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .

# Build the application
RUN cargo install --path . && rm -rf target

# Runtime stage
# We use debian:bookworm-slim for a small footprint while maintaining glibc compatibility
FROM debian:bookworm-slim

# Install necessary runtime dependencies (if any, usually none for simple actix app but ca-certificates is good practice)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/app-template-backend /usr/local/bin/app-template-backend

# Expose the port
EXPOSE 9876

# Run the binary
CMD ["app-template-backend"]
