# Pocket Arcade

Classic browser games written in Rust and compiled to WebAssembly.

## Run it

```sh
scripts/build.sh      # compile every game to web/pkg/
scripts/serve.sh      # http://localhost:8080
cargo test            # game-logic unit tests (run natively, no browser)
```

Needs: Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`.

## Layout

```
crates/engine/        shared: Game trait, export_game! macro, canvas helpers, input, rng
crates/games/snake/   logic.rs (rules + tests), draw.rs (retro + modern), lib.rs (glue)
crates/games/bricks/  same shape
web/                  static site: menu, theme toggle, HUD, high scores (localStorage)
web/js/games.js       the game catalogue
```

Rust owns the rules and the drawing. JavaScript only does the page:
routing, the game loop, keyboard events, the HUD and saved scores.

## Adding a game

1. Copy `crates/games/snake` to `crates/games/<id>` and rename the package.
2. Implement `engine::Game` (logic, `render` for both themes) and call `engine::export_game!(YourType)`.
3. Add an entry to `web/js/games.js`.
4. `scripts/build.sh`.

## Deploying

Live at https://games.engineeradriel.com (GitHub Pages).

- Every push to `main` runs `.github/workflows/deploy.yml`: tests, builds the
  WebAssembly, and publishes `web/`.
- DNS: `infra/dns.yaml` (CloudFormation) adds the `games` CNAME to the
  engineeradriel.com Route 53 zone. One-time setup:

```sh
aws cloudformation deploy --profile <profile> --stack-name games-dns \
  --template-file infra/dns.yaml --parameter-overrides HostedZoneId=<zone-id>
```
