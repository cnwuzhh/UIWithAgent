set dotenv-load := false

bootstrap:
    pnpm install

build:
    pnpm build
    cargo check --workspace

dev:
    pnpm dev

test:
    cargo test --workspace
    pnpm typecheck
