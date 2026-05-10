# Day 12 — Snake

**Domain:** games • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

The classic game of Snake. A segmented snake moves automatically through a bounded playfield; you steer it with arrow keys; it eats food, grows longer, speeds up slightly, dies on collision with itself or the walls. Game-over screen, restart, high-score tracking, pause. By the end of today you've got a real arcade game you'd be happy to open-source.

## What you'll learn

- The **game-loop** pattern
- **Fixed-timestep updates** — why the snake moves at a consistent speed regardless of framerate
- **Non-blocking input** with `event::poll` + `Duration`
- Decoupling **game logic**, **rendering**, and **input** into separate modules
- Basic **state machine** (Playing / Paused / Over)
- Simple file persistence for a high score

## Background

### The naive game loop

```rust
loop {
    handle_input();
    update();
    render();
    sleep(16ms);
}
```

Works for a toy. Breaks on real hardware. Problems:

- On a slow machine, the loop falls behind — the snake moves slower.
- On a fast one, the loop runs too fast — the snake flies.
- `sleep(16ms)` sleeps *at least* 16ms; actual duration varies.

### Fixed-timestep loops

Separate "what time is it?" from "how often should updates fire?". Updates happen at a fixed rate (say, every 150 ms). Rendering is unlocked — as fast as the machine can go, but not more often than needed.

```rust
let tick = Duration::from_millis(150);
let mut last = Instant::now();
let mut accumulator = Duration::ZERO;

loop {
    let now = Instant::now();
    accumulator += now - last;
    last = now;

    while accumulator >= tick {
        update();
        accumulator -= tick;
    }

    render();
    poll_input_for_up_to(Duration::from_millis(1));
}
```

On a fast machine, the `while` runs zero or one time per outer iteration — render at high framerate, update at 150 ms intervals. On a slow machine, `accumulator` may be 300 ms by the time we check — we run two updates to catch up, then render. The snake always moves at 150 ms per step regardless of hardware.

This is **the** standard pattern for real-time games. Read Glenn Fiedler's "Fix Your Timestep" if you want the full treatment — for us the simple version is enough.

### Non-blocking input

For the adventure game, blocking was fine — nothing happens until the player types. Snake is different: the snake moves *regardless* of input. We use `event::poll(duration)` — returns true if input is available within the duration. If not, move on.

### Three modules, three concerns

Keep game logic separate from rendering and input:

- `game.rs` — pure logic. No I/O, no crossterm. Just types and state transitions. Can be tested offline.
- `render.rs` — takes a `&Game`, prints it with crossterm.
- `main.rs` — the loop. Timing. Orchestration.

## Setting up

```bash
cargo new day-12
cd day-12
cargo add crossterm rand@0.8
```

## Step 1 — Game types

Create `src/game.rs`:

```rust
use std::collections::VecDeque;

pub const WIDTH: i32 = 30;
pub const HEIGHT: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up, Down, Left, Right,
}

impl Direction {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up    => (0, -1),
            Direction::Down  => (0,  1),
            Direction::Left  => (-1, 0),
            Direction::Right => (1,  0),
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::Up    => Direction::Down,
            Direction::Down  => Direction::Up,
            Direction::Left  => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Over,
}

pub struct Game {
    pub snake: VecDeque<(i32, i32)>,   // head at front
    pub direction: Direction,
    pub pending_dir: Direction,         // applied on next tick
    pub food: (i32, i32),
    pub status: Status,
    pub score: u32,
    pub high_score: u32,
    pub tick_ms: u64,
}
```

### Why `VecDeque` for the snake?

Every tick: add new head at the front, remove old tail. `Vec::insert(0, ...)` and `Vec::remove(0)` are O(n). `VecDeque` does both in O(1).

### Two direction fields?

`direction` is the currently-active heading. `pending_dir` is what the player last pressed. We only commit `pending_dir` to `direction` at the start of each tick. This prevents the snake from 180°-ing on itself in a single tick (double-press Left then Right would otherwise collapse the snake into its own body).

## Step 2 — Construction & spawning food

```rust
use rand::Rng;

impl Game {
    pub fn new(high_score: u32) -> Game {
        let mut snake = VecDeque::new();
        let start_x = WIDTH / 2;
        let start_y = HEIGHT / 2;
        snake.push_back((start_x, start_y));
        snake.push_back((start_x - 1, start_y));
        snake.push_back((start_x - 2, start_y));

        let mut game = Game {
            snake,
            direction: Direction::Right,
            pending_dir: Direction::Right,
            food: (0, 0),
            status: Status::Playing,
            score: 0,
            high_score,
            tick_ms: 150,
        };
        game.spawn_food();
        game
    }

    pub fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let pos = (rng.gen_range(1..WIDTH - 1), rng.gen_range(1..HEIGHT - 1));
            if !self.snake.contains(&pos) {
                self.food = pos;
                return;
            }
        }
    }
}
```

### Why the loop in `spawn_food`?

Food can't spawn on the snake. Could use a clever approach (enumerate all empty cells, pick one), but for small boards this is fine and simple: keep rolling until we hit an empty cell.

## Step 3 — Input handling

```rust
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Turn(Direction),
    Pause,
    Restart,
    Quit,
}

impl Game {
    pub fn apply_input(&mut self, input: Input) {
        match input {
            Input::Turn(dir) => {
                if self.status == Status::Playing {
                    // Reject 180° reversals
                    if dir != self.direction.opposite() {
                        self.pending_dir = dir;
                    }
                }
            }
            Input::Pause => {
                self.status = match self.status {
                    Status::Playing => Status::Paused,
                    Status::Paused => Status::Playing,
                    Status::Over => Status::Over,
                };
            }
            Input::Restart => {
                if self.status == Status::Over {
                    *self = Game::new(self.high_score);
                }
            }
            Input::Quit => {}   // handled by main loop
        }
    }
}
```

Input comes from outside as a typed enum. Logic stays pure — no direct key handling here.

## Step 4 — The update

```rust
impl Game {
    pub fn update(&mut self) {
        if self.status != Status::Playing {
            return;
        }

        // Commit the pending direction
        self.direction = self.pending_dir;

        let (dx, dy) = self.direction.delta();
        let head = *self.snake.front().expect("snake must have a head");
        let new_head = (head.0 + dx, head.1 + dy);

        // Wall collision
        if new_head.0 <= 0 || new_head.0 >= WIDTH - 1
            || new_head.1 <= 0 || new_head.1 >= HEIGHT - 1
        {
            self.status = Status::Over;
            self.high_score = self.high_score.max(self.score);
            return;
        }

        // Self collision
        //   (but allow moving into the tail spot, because the tail is about to vanish —
        //    except when we're about to eat, so tail doesn't vanish)
        let eating = new_head == self.food;
        let skip_last = if eating { 0 } else { 1 };
        for seg in self.snake.iter().take(self.snake.len() - skip_last) {
            if *seg == new_head {
                self.status = Status::Over;
                self.high_score = self.high_score.max(self.score);
                return;
            }
        }

        // Grow / move
        self.snake.push_front(new_head);
        if eating {
            self.score += 10;
            if self.tick_ms > 60 && self.score % 50 == 0 {
                self.tick_ms -= 5;    // speed up every 50 points
            }
            self.spawn_food();
        } else {
            self.snake.pop_back();
        }
    }
}
```

### The tail-trailing edge case

Subtle. When the snake moves without eating, its tail vacates a cell. If the new head is where the tail *was*, that's a legal move — you're following your own tail. But when eating, the tail doesn't leave; a move into a body segment is death.

We handle this by iterating all body segments *except* the last (if not eating) when checking self-collision.

## Step 5 — Render

Create `src/render.rs`:

```rust
use crate::game::{Direction, Game, Status, HEIGHT, WIDTH};
use crossterm::{cursor, queue, style::{self, Color}};
use std::io::{self, Write};

pub fn render(game: &Game, stdout: &mut impl Write) -> io::Result<()> {
    queue!(stdout, cursor::MoveTo(0, 0))?;

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = cell_char(game, x, y);
            let color = cell_color(game, x, y);
            queue!(
                stdout,
                style::SetForegroundColor(color),
                style::Print(c),
                style::ResetColor,
            )?;
        }
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    // Status line
    let status_text = match game.status {
        Status::Playing => format!(
            "Score: {}   High: {}   Speed: {}ms   q:quit p:pause",
            game.score, game.high_score, game.tick_ms,
        ),
        Status::Paused => format!(
            "PAUSED   Score: {}   q:quit p:resume",
            game.score,
        ),
        Status::Over => format!(
            "GAME OVER   Score: {}   High: {}   r:restart q:quit",
            game.score, game.high_score,
        ),
    };
    queue!(
        stdout,
        cursor::MoveToNextLine(1),
        style::Print(status_text),
        style::Print(" ".repeat(20)),   // pad to overwrite previous content
    )?;

    stdout.flush()?;
    Ok(())
}

fn cell_char(game: &Game, x: i32, y: i32) -> char {
    if x == 0 || x == WIDTH - 1 || y == 0 || y == HEIGHT - 1 {
        '#'
    } else if let Some(front) = game.snake.front() {
        if *front == (x, y) {
            '@'
        } else if game.snake.contains(&(x, y)) {
            'o'
        } else if game.food == (x, y) {
            '*'
        } else {
            ' '
        }
    } else {
        ' '
    }
}

fn cell_color(game: &Game, x: i32, y: i32) -> Color {
    if x == 0 || x == WIDTH - 1 || y == 0 || y == HEIGHT - 1 {
        Color::Cyan
    } else if game.snake.contains(&(x, y)) {
        Color::Green
    } else if game.food == (x, y) {
        Color::Yellow
    } else {
        Color::Reset
    }
}
```

Nothing fancy — just a double-dispatch (char and color per cell). `cell_char` checks the borders first, then the snake/food, falling back to space. `cell_color` does the same for color.

## Step 6 — Main loop

`src/main.rs`:

```rust
mod game;
mod render;

use game::{Direction, Game, Input, Status};
use render::render;

use crossterm::{
    cursor, execute,
    event::{self, Event, KeyCode},
    terminal,
};
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

struct RawGuard;
impl RawGuard {
    fn new() -> io::Result<RawGuard> {
        terminal::enable_raw_mode()?;
        Ok(RawGuard)
    }
}
impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            cursor::Show,
        );
        let _ = terminal::disable_raw_mode();
    }
}

const HIGH_SCORE_PATH: &str = "high_score.dat";

fn read_high_score() -> u32 {
    std::fs::read_to_string(HIGH_SCORE_PATH)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_high_score(score: u32) {
    let _ = std::fs::write(HIGH_SCORE_PATH, score.to_string());
}

fn poll_input() -> io::Result<Option<Input>> {
    if !event::poll(Duration::from_millis(1))? {
        return Ok(None);
    }
    let Event::Key(key) = event::read()? else { return Ok(None); };
    let input = match key.code {
        KeyCode::Up    => Input::Turn(Direction::Up),
        KeyCode::Down  => Input::Turn(Direction::Down),
        KeyCode::Left  => Input::Turn(Direction::Left),
        KeyCode::Right => Input::Turn(Direction::Right),
        KeyCode::Char('p') => Input::Pause,
        KeyCode::Char('r') => Input::Restart,
        KeyCode::Char('q') | KeyCode::Esc => Input::Quit,
        _ => return Ok(None),
    };
    Ok(Some(input))
}

fn main() -> io::Result<()> {
    let _guard = RawGuard::new()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide,
    )?;

    let high_score = read_high_score();
    let mut game = Game::new(high_score);

    let mut last = Instant::now();
    let mut accumulator = Duration::ZERO;

    loop {
        let now = Instant::now();
        accumulator += now - last;
        last = now;

        // Handle all pending input
        while let Some(input) = poll_input()? {
            if matches!(input, Input::Quit) {
                if game.score > 0 {
                    write_high_score(game.high_score);
                }
                return Ok(());
            }
            game.apply_input(input);
        }

        // Fixed-timestep updates
        let tick = Duration::from_millis(game.tick_ms);
        while accumulator >= tick {
            game.update();
            accumulator -= tick;
            // Persist high score on game over
            if game.status == Status::Over {
                write_high_score(game.high_score);
            }
        }

        // Render
        render(&game, &mut stdout)?;

        // Yield briefly if we're way ahead of schedule
        if accumulator < tick / 2 {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
```

### Reading this carefully

- **Input processing** drains everything currently queued. If the player rapid-fires arrow keys, we process them all without waiting for a tick.
- **`while accumulator >= tick`** — the catch-up loop. On a fast machine, runs 0 or 1 times. On a slow one, more.
- **`game.tick_ms`** can change as the player scores — so we recompute `tick` each iteration.
- **Sleep when ahead** — if we've got time to spare, sleep 5ms. Prevents the loop from burning 100% CPU.

## Step 7 — Play

```bash
cargo run --release
```

Always use `--release` for real games. Debug builds are 10–100× slower.

You should get a bordered playfield with your snake starting in the middle, a yellow food dot somewhere, and responsive arrow-key controls. Eat food to grow. Speed up every 50 points. Hit a wall or yourself to die. Press `r` to restart, `q` to quit.

The high score persists across runs in `high_score.dat` in the project directory.

## Step 8 — A quick test of the logic (preview)

Because `game.rs` has no I/O dependency, it's testable offline. Add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_moves_right() {
        let mut game = Game::new(0);
        // Head starts at (15, 10), direction right
        let head_before = *game.snake.front().unwrap();
        game.update();
        let head_after = *game.snake.front().unwrap();
        assert_eq!(head_after, (head_before.0 + 1, head_before.1));
    }

    #[test]
    fn cant_reverse() {
        let mut game = Game::new(0);
        // Snake is moving Right
        game.apply_input(Input::Turn(Direction::Left));
        // Direction should not have changed
        assert_eq!(game.pending_dir, Direction::Right);
    }

    #[test]
    fn wall_collision_ends_game() {
        let mut game = Game::new(0);
        // Move right until we hit the wall
        for _ in 0..WIDTH {
            game.update();
        }
        assert_eq!(game.status, Status::Over);
    }
}
```

Run:

```bash
cargo test
```

All pass. Day 13 makes testing a first-class topic.

## Common pitfalls

### The snake "teleports" diagonally

You're committing `pending_dir` to `direction` *during* input, not at the start of the tick. Rapid Up + Right input causes two direction changes before the update sees the snake. Fix: commit only at the top of `update`.

### The snake 180°s

Same root cause. Plus, your opposite-reversal check happens against `direction` before the commit. If the snake is moving Right and you press Left, `pending_dir.opposite()` needs to compare to the *current* direction. Our code does — verify yours does too.

### Food spawns on the snake

Your spawn loop didn't check. With `contains`, verify. For huge snakes on small boards, this gets slow — generate an empty-cell list instead.

### Game runs at different speeds on different machines

`sleep(tick_ms)` is duration between updates, not between events. On a fast machine the sleep is accurate, on a slow one updates pile up, resulting in bursty motion. Fixed-timestep pattern fixes this.

### Terminal flickers

You called `render` from inside the `update` loop. Update can run multiple times per outer iteration; rendering multiple times per frame causes flicker. Render exactly once per outer loop iteration.

### High score not saved

The program exits via Ctrl-C or `q` before writing. Our code writes on quit and on game-over. Make sure both paths are covered; use `RawGuard::drop` to persist if necessary — but be careful: panics in drop are bad, so use `let _ = write(...)` to ignore errors.

### Snake flies on fast machines

You didn't use `--release`. Debug mode runs so slowly the update rarely triggers; release runs accurately. Always benchmark and play in release.

## What you learned

- **Fixed-timestep game loops** — the standard real-time pattern.
- **Non-blocking input** via `event::poll(Duration)`.
- Separating **pure game logic** from I/O — enables testing.
- **State machine** via `Status` enum (Playing / Paused / Over).
- `VecDeque` for snake bodies (O(1) push/pop from both ends).
- Simple file persistence for settings like high score.
- Speed ramp by adjusting tick interval.

## Exercises

1. **Wrap-around mode.** Instead of dying on walls, the snake wraps to the opposite side. Make it a compile-time flag or a menu option.
2. **Two-player.** Add a second snake controlled by WASD. Collision between snakes = the colliding one dies. Screen splits or they share the board.
3. **Obstacles.** Procedurally scatter a few walls in the playfield. Each new game, new layout.
4. **Difficulty menu.** On first run, a menu lets you pick Easy (tick 200ms, no speed-up), Medium (default), Hard (tick 80ms, faster ramp).
5. **Persistent stats.** Track total games played, total food eaten, longest snake. Serialize to a small JSON file (preview of Day 16).

## What's next

Day 13 is dedicated to **testing**. You've written a few `#[test]` functions here and there; tomorrow you'll meet the full toolkit: unit tests, integration tests, doctests, property-based tests with `proptest`, and benchmarks with `criterion`. Your Snake and parser will get proper test suites.

→ [Day 13 — Testing](day-13.md)
