FROM rust:1.94-trixie AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock build.rs ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml
COPY shared/Cargo.toml shared/Cargo.toml
COPY . .

RUN cargo build --locked --release -p backend

FROM debian:trixie-slim AS runtime

RUN apt-get update \
	&& apt-get install --yes --no-install-recommends ca-certificates \
	&& useradd --system --create-home --uid 10001 sprouts \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /workspace/target/release/backend /usr/local/bin/backend

ENV APP_ENV=production
ENV BIND_ADDRESS=0.0.0.0:3000

USER sprouts

EXPOSE 3000

CMD ["backend"]
