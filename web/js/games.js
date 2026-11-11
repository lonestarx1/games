// The game catalogue. To add a game: create crates/games/<id>, run
// scripts/build.sh, and add an entry here. Every game module exports the same
// `Game` class (see crates/engine/src/lib.rs, `export_game!`).
export const GAMES = [
  {
    id: "snake",
    title: "Snake",
    blurb: "Eat, grow, and don't bite your own tail. Walls wrap around.",
    help: "Arrow keys steer · Space pauses",
    start: "Press an arrow key or Space",
    module: "../pkg/snake/snake.js",
  },
  {
    id: "bricks",
    title: "Bricks",
    blurb: "Bounce the ball up into the wall. Clear it to reach the next, faster level.",
    help: "← → move · Space launches and pauses",
    start: "Press Space to launch",
    module: "../pkg/bricks/bricks.js",
  },
  {
    id: "mahjong",
    title: "Mahjong",
    blurb: "Match pairs of identical free tiles until the pyramid is gone.",
    help: "Click two identical free tiles · H hint · Z undo · R shuffle · N new · Space pause",
    start: "Click a tile to begin",
    module: "../pkg/mahjong/mahjong.js",
    pointer: true,
    // Buttons under the board; each sends a key code to the game.
    actions: [
      { label: "Hint", code: "KeyH" },
      { label: "Undo", code: "KeyZ" },
      { label: "Shuffle", code: "KeyR" },
      { label: "New game", code: "KeyN" },
    ],
  },
];
