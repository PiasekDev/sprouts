# syntax=docker/dockerfile:1

FROM rust:1.94-trixie AS builder

WORKDIR /workspace
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache

RUN apt-get update \
	&& apt-get install --yes --no-install-recommends sccache \
	&& rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
	--mount=type=cache,target=/usr/local/cargo/git/db,sharing=locked \
	--mount=type=cache,target=/sccache,sharing=locked \
	cargo install --locked trunk \
	&& rustup target add wasm32-unknown-unknown

COPY Cargo.toml Cargo.lock build.rs ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml
COPY shared/Cargo.toml shared/Cargo.toml
COPY . .

WORKDIR /workspace/frontend

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
	--mount=type=cache,target=/usr/local/cargo/git/db,sharing=locked \
	--mount=type=cache,target=/sccache,sharing=locked \
	trunk build --release

FROM nginx:1.29.6-alpine3.23 AS runtime

COPY frontend/server.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /workspace/frontend/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
