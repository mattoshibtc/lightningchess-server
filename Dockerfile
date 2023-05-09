FROM rust:1.64.0 as builder
WORKDIR /app
COPY . .
RUN cargo install --profile release --path .

FROM debian:buster-slim as runner
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget gcc libssl-dev libc6-dev
COPY --from=builder /usr/local/cargo/bin/lightningchess /usr/local/bin/lightningchess
COPY --from=builder /app/Rocket.toml ./Rocket.toml
COPY --from=builder /app/templates/index.html.hbs ./templates/index.html.hbs
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["lightningchess"]