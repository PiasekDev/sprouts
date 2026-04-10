FROM rust:1.94-trixie AS builder

WORKDIR /workspace

RUN cargo install --locked trunk \
	&& rustup target add wasm32-unknown-unknown

COPY Cargo.toml Cargo.lock build.rs ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml
COPY shared/Cargo.toml shared/Cargo.toml
COPY . .

WORKDIR /workspace/frontend

RUN trunk build --release

FROM nginx:1.27-alpine AS runtime

COPY frontend/server.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /workspace/frontend/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
