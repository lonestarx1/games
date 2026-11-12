// The game catalogue. To add a game: create crates/games/<id>, run
// scripts/build.sh, and add an entry here. Every game module exports the same
// `Game` class (see crates/engine/src/lib.rs, `export_game!`).
//
// `howto` is the beginner's "How to play" card: a one-paragraph goal, the
// rules, a controls table ([keys, what they do]) and a few tips. It opens
// automatically the first time someone plays a game.
export const GAMES = [
  {
    id: "snake",
    title: "Snake",
    blurb: "Eat, grow, and don't bite your own tail. Walls wrap around.",
    help: "Arrow keys steer · Space pauses",
    start: "Press an arrow key or Space",
    module: "../pkg/snake/snake.js",
    howto: {
      goal: "Steer the snake to the red food. Every bite makes it longer and a little faster. Don't let its head touch its own body.",
      rules: [
        "The snake keeps moving by itself. You only choose which way it turns.",
        "It can't turn straight back the way it came.",
        "Leaving one edge brings you back in on the opposite edge.",
        "Each food is worth 10 points. The game ends when the head hits the body.",
      ],
      controls: [
        ["Arrow keys or W A S D", "Turn"],
        ["Space or P", "Pause and resume"],
      ],
      tips: [
        "Early on, move in wide loops so the middle stays open.",
        "When the snake is long, follow the edges and always leave yourself a way out.",
      ],
    },
  },
  {
    id: "bricks",
    title: "Bricks",
    blurb: "Bounce the ball up into the wall. Clear it to reach the next, faster level.",
    help: "← → move · Space launches and pauses",
    start: "Press Space to launch",
    module: "../pkg/bricks/bricks.js",
    howto: {
      goal: "Bounce the ball off your paddle to smash every brick in the wall. Clear the wall to reach the next level, where the ball moves faster.",
      rules: [
        "The ball starts on your paddle. Launch it, then don't let it fall past you.",
        "Where the ball hits the paddle sets its direction: the middle sends it straight up, the edges send it off at an angle.",
        "Bricks in higher rows are worth more points.",
        "You have one ball. If it falls off the bottom, the game ends.",
      ],
      controls: [
        ["← → or A D", "Move the paddle"],
        ["Space or ↑", "Launch the ball"],
        ["Space or P", "Pause and resume"],
      ],
      tips: [
        "Use the paddle's edges to aim at gaps. A ball that gets above the wall clears lots of bricks on its own.",
      ],
    },
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
    howto: {
      goal: "Clear all 144 tiles from the board by removing them in matching pairs. This is a solo puzzle, not the four-player game.",
      rules: [
        "Two tiles match only if they look exactly the same.",
        "You can only pick a free tile: nothing on top of it, and its left or right side open.",
        "Click a free tile, then its twin. The pair disappears.",
        "You get 10 points per pair, a bonus for quick matches in a row, and a big bonus for clearing the board fast.",
        "Every deal can be solved. If you run out of moves, shuffle the tiles that are left.",
      ],
      controls: [
        ["Click", "Pick a tile"],
        ["H", "Show a hint"],
        ["Z", "Undo the last pair"],
        ["R", "Shuffle the remaining tiles"],
        ["N", "Start a new deal"],
        ["Space", "Pause and resume"],
      ],
      tips: [
        "Work on the tall middle stack and the ends of long rows early. They unlock the most tiles.",
        "When three or four of the same tile are free, pick the pair that frees up more tiles.",
      ],
    },
  },
  {
    id: "blockdrop",
    title: "Block Drop",
    blurb: "Turn and slide falling shapes to fill whole rows. Watch for bombs. It speeds up as you go.",
    help: "← → move · ↑ turn · ↓ faster · Space drop · C hold · P pause",
    start: "Press any arrow key or Space",
    module: "../pkg/blockdrop/blockdrop.js",
    actions: [
      { label: "◀", code: "ArrowLeft" },
      { label: "Turn", code: "ArrowUp" },
      { label: "▶", code: "ArrowRight" },
      { label: "▼", code: "ArrowDown" },
      { label: "Drop", code: "Space" },
      { label: "Hold", code: "KeyC" },
    ],
    howto: {
      goal: "Shapes made of four blocks fall into the well one at a time. Move and turn them so they fill complete rows. A full row disappears and scores points. The game ends when the pile reaches the top.",
      rules: [
        "The panel on the right shows the next three shapes.",
        "Every 8th shape carries a bomb. When it lands, it blasts away the 3 × 3 square around it, and blocks above the hole fall in. Each block destroyed scores 10 points times your level.",
        "A shape locks shortly after it lands, so you still have a moment to slide or turn it.",
        "Clearing several rows with one shape scores far more: 1 row 100, 2 rows 300, 3 rows 500, 4 rows 800. All of these are multiplied by your level.",
        "Every 10 rows you clear raises the level. The shapes fall faster and the colours change.",
        "Hold sets the current shape aside to use later. You can hold once per shape.",
      ],
      controls: [
        ["← →", "Move left or right (hold to slide)"],
        ["↑ or X", "Turn clockwise"],
        ["Z", "Turn the other way"],
        ["↓", "Fall faster while held"],
        ["Space", "Drop straight down"],
        ["C", "Hold or swap the shape"],
        ["P or Esc", "Pause and resume"],
      ],
      tips: [
        "The light beam under the falling shape shows which columns it will drop into. The bright marks show where it will land.",
        "Use a bomb to dig out a block that's covering a hole.",
        "Keep the pile flat, and leave one column empty so the long bar can clear four rows at once.",
      ],
    },
  },
  {
    id: "minefinder",
    title: "Mine Finder",
    blurb: "Use the number clues to open every safe square without touching a mine.",
    help: "Click opens · Right-click or long-press flags · Arrows + Space + F also work",
    start: "Click any square to start",
    module: "../pkg/minefinder/minefinder.js",
    pointer: true,
    longPress: true,
    actions: [
      { label: "Easy", code: "KeyE" },
      { label: "Medium", code: "KeyM" },
      { label: "Hard", code: "KeyH" },
      { label: "Flag mode", code: "KeyT" },
      { label: "New game", code: "KeyN" },
    ],
    howto: {
      goal: "Mines are hidden under some of the squares. Open every square that doesn't have a mine. Open a mine and you lose.",
      rules: [
        "Your first square is always safe and opens up an area.",
        "A number tells you how many mines touch that square, counting all 8 around it, diagonals included.",
        "A blank square has no mines around it, so its neighbours open by themselves.",
        "When you're sure a square hides a mine, put a flag on it. Flagged squares can't be opened by accident.",
        "Clicking a number that already has that many flags around it opens all its other neighbours at once.",
        "Easy is 9 × 9 with 10 mines, Medium is 16 × 16 with 40, Hard is 24 × 20 with 99. Harder boards score more, and finishing fast earns a bonus.",
      ],
      controls: [
        ["Click or tap", "Open a square"],
        ["Right-click or long-press", "Place or remove a flag"],
        ["Flag mode button", "Make taps place flags (handy on phones)"],
        ["Arrow keys", "Move the cursor"],
        ["Space", "Open the square under the cursor"],
        ["F", "Flag the square under the cursor"],
        ["E, M, H", "New Easy, Medium or Hard board"],
        ["P", "Pause the clock"],
      ],
      tips: [
        "Start with the 1s. A 1 touching only one closed square means that square is a mine.",
        "If a number already has all its mines flagged, every other square around it is safe.",
        "Corners and edges have fewer neighbours, so their clues are often easier to read.",
      ],
    },
  },
];
