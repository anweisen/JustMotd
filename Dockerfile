# Stage 1: Build the Rust binary
FROM rust:latest as build

WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files to the container
COPY Cargo.toml ./
COPY src/ src/

# Build the Rust application
RUN cargo build --release

# Stage 2: Execute the binary on a minimal Linux server image
FROM debian:bookworm-slim

WORKDIR /app

# Copy the compiled binary from the builder stage to the final stage
COPY --from=build /app/target/release/justmotd .

EXPOSE 25565

# Define the command to run when the container starts
CMD ["./justmotd"]
