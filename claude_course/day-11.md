# Day 11 — Terminal Rendering

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** medium

## What you'll build

A "walk around" demo. A 40×15 grid of ASCII walls, floors, and a player character `@`. Arrow keys move the player; walls block movement; `q` quits. No game logic yet — just rendering + input plumbing. Tomorrow you build Snake on top.

## What you'll learn

- How terminals work: **raw mode**, **ANSI escape codes**, the screen buffer
- The **`crossterm`** crate — cross-platform terminal control
- **Cursor positioning**, **clearing**, **colors**, **hiding the cursor**
- **Blocking input** via `event::read`
- **RAII guards** via `Drop` for guaranteed cleanup
- Flushing a frame in one shot to avoid flicker

## Background

### What is a terminal, really?

A terminal is a text grid. Programs write bytes to stdout; most bytes are characters, but some sequences are **escape codes** that the terminal interprets as commands: move the cursor, change color, clear the screen, ring the bell.

```
\x1b[2J         clear the screen
\x1b[H          move cursor to top-left
\x1b[31m        make subsequent text red
\x1b[0m         reset color
```

`\x1b` is the escape character (ASCII 27). `[` starts a control sequence. Numbers and letters form specific commands.

You could write these by hand. You almost never should.

### Cooked mode vs raw mode

By default, terminals run in **cooked mode**: your program doesn't see individual keystrokes. Input is line-buffered — the terminal holds onto what you type until you press Enter, then delivers the whole line. Also: Ctrl-C still kills your process; the terminal echoes what you type; backspace works.

For games you need **raw mode**: each keystroke arrives immediately, as-is. No line buffering. No automatic echo. Ctrl-C doesn't kill you (you have to handle it yourself).

Raw mode is a global terminal state. If your program exits without restoring cooked mode, the user's shell is broken until they run `reset`. You must cleanly restore it on exit, including on panic. That's what RAII guards are for.

### Crossterm

Linux uses one set of escape codes; Windows uses a completely different API (Win32 console). Writing either by hand is miserable; writing cross-platform manually is worse.

The `crossterm` crate abstracts all this. One API, works everywhere. Raw mode, cursor movement, colors, events — all via Rust functions.

### What does `execute!` do?

```rust
use crossterm::{execute, cursor, terminal};

execute!(
    io::stdout(),
    terminal::Clear(terminal::ClearType::All),
    cursor::MoveTo(0, 0),
)?;
```

`execute!` is a macro that writes one or more commands to the given output and flushes. Each command is a struct implementing crossterm's `Command` trait, which knows how to render itself as platform-appropriate bytes.

Alternative: `queue!` writes commands *without* flushing. Useful for building up a whole frame before flushing once.

## Setting up

```bash
cargo new day-11
cd day-11
cargo add crossterm
```

Verify `Cargo.toml`:

```toml
[dependencies]
crossterm = "0.27"
```

## Step 1 — Enter raw mode, clear screen, exit cleanly

Start `main.rs`:

```rust
use std::io::{self, Write};
use crossterm::{cursor, execute, terminal};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide,
    )?;

    write!(stdout, "Hello from raw mode! Press any key...")?;
    stdout.flush()?;

    // Block until a key is pressed
    crossterm::event::read()?;

    execute!(stdout, cursor::Show, cursor::MoveTo(0, 0))?;
    terminal::disable_raw_mode()?;

    println!("Goodbye.");
    Ok(())
}
```

Run it. You should see the screen clear, "Hello from raw mode!" appear, then wait for a keypress. Press any key and it exits cleanly.

### A problem waiting to happen

What if the program panics between `enable_raw_mode` and `disable_raw_mode`? The terminal is left in raw mode. The user's shell is broken — no echo, Ctrl-C doesn't work. They have to type `reset` blind.

Next step: fix this properly.

## Step 2 — RAII guard for cleanup

The classic Rust pattern: a struct whose `Drop` impl does the cleanup. No matter how the function exits — normal return, early `?`, panic — `Drop` runs.

```rust
struct RawGuard;

impl RawGuard {
    fn new() -> io::Result<RawGuard> {
        terminal::enable_raw_mode()?;
        Ok(RawGuard)
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        // Best-effort cleanup; we can't return errors from Drop
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), cursor::Show);
    }
}
```

Updated main:

```rust
fn main() -> io::Result<()> {
    let _guard = RawGuard::new()?;

    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide,
    )?;

    write!(stdout, "Hello with guard! Press any key...")?;
    stdout.flush()?;

    crossterm::event::read()?;
    Ok(())
}
```

Now if anything panics — or if we forget to restore — the `Drop` still runs. Try forcing a panic:

```rust
crossterm::event::read()?;
panic!("oops");   // raw mode will still be disabled
Ok(())
```

Run it. Panic happens, stack unwinds, `_guard` drops, raw mode is off, terminal is usable. Exactly what we want.

### Why `let _guard = ...`?

Not `let _ = ...`. The underscore-prefix name `_guard` *does* create a binding that lives for the scope. Plain `_` would drop the guard immediately — cleanup would run right after creation, defeating the purpose.

This is a common trap. If you see `let _foo = ...`, that variable *is* alive till end of scope. `let _ = ...` drops right away.

## Step 3 — The grid

A simple owned 2D char buffer:

```rust
pub struct Grid {
    pub width: usize,
    pub height: usize,
    cells: Vec<char>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            cells: vec!['.'; width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> char {
        self.cells[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, c: char) {
        self.cells[y * self.width + x] = c;
    }
}
```

Row-major storage: index = `y * width + x`. Standard.

### Load a map from a string literal

```rust
pub fn from_str(map: &[&str]) -> Grid {
    let height = map.len();
    let width = map.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut grid = Grid::new(width, height);
    for (y, row) in map.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            grid.set(x, y, c);
        }
    }
    grid
}
```

Usage:

```rust
let map = [
    "########################################",
    "#......................................#",
    "#...####...........####................#",
    "#...#..#...............#...............#",
    "#...####...............####............#",
    "#......................................#",
    "#.................####.................#",
    "#.................#..#.................#",
    "#......................................#",
    "########################################",
];
let grid = Grid::from_str(&map);
```

Ten rows, walls (`#`), floors (`.`).

## Step 4 — Render

Render the grid to the terminal. Key principle: build the full frame in a string, then write once. Per-cell `execute!` flickers badly.

```rust
use std::io::Write;

pub fn render(grid: &Grid, player: (usize, usize), stdout: &mut impl Write) -> io::Result<()> {
    use crossterm::{cursor, queue, style};

    queue!(stdout, cursor::MoveTo(0, 0))?;

    for y in 0..grid.height {
        for x in 0..grid.width {
            let c = if (x, y) == player { '@' } else { grid.get(x, y) };
            queue!(stdout, style::Print(c))?;
        }
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    stdout.flush()?;
    Ok(())
}
```

### Why `queue!` over `execute!`?

`queue!` writes commands without flushing. We accumulate the whole frame in stdout's buffer, then `flush()` once at the end. That's one syscall per frame instead of one per cell. Flicker-free rendering.

### `cursor::MoveToNextLine(1)`

After printing a row, the cursor is at column `width`. We need to wrap to column 0 of the next row. On a normal terminal, a newline (`\n`) does this — but in raw mode, `\n` moves down without returning to column 0 (you'd need `\r\n`). `MoveToNextLine(n)` works correctly everywhere.

### Test it

Put it all together:

```rust
fn main() -> io::Result<()> {
    let _guard = RawGuard::new()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All), cursor::Hide)?;

    let map = [
        "########################################",
        "#......................................#",
        "#...####...............................#",
        "#...#..#...............................#",
        "#...####...............................#",
        "#......................................#",
        "#......................................#",
        "########################################",
    ];
    let grid = Grid::from_str(&map);

    let player = (5, 3);
    render(&grid, player, &mut stdout)?;

    crossterm::event::read()?;   // wait for keypress
    Ok(())
}
```

Run it. You should see:

```
########################################
#......................................#
#...####...............................#
#...#@.#...............................#
#...####...............................#
#......................................#
#......................................#
########################################
```

with `@` where the player sits.

## Step 5 — Input handling

Now arrow keys. `crossterm::event::read()` blocks until an event arrives, returning `Event::Key(KeyEvent)` for key presses.

```rust
use crossterm::event::{self, Event, KeyCode};

pub fn handle_input(grid: &Grid, player: &mut (usize, usize)) -> io::Result<bool> {
    // Returns Ok(true) to continue, Ok(false) to quit
    match event::read()? {
        Event::Key(key_event) => {
            let (dx, dy) = match key_event.code {
                KeyCode::Up    => (0, -1),
                KeyCode::Down  => (0,  1),
                KeyCode::Left  => (-1, 0),
                KeyCode::Right => (1,  0),
                KeyCode::Char('q') => return Ok(false),
                _ => return Ok(true),
            };
            let (x, y) = *player;
            let nx = (x as i32 + dx) as usize;
            let ny = (y as i32 + dy) as usize;
            if grid.get(nx, ny) != '#' {
                *player = (nx, ny);
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}
```

Watch the `as` casts: we briefly go to `i32` to compute the delta, then back to `usize` to index. If the player tried to move off the grid, `nx` or `ny` could underflow — but we bounded the map with walls, so every valid move stays inside. Still, a real game would bounds-check.

### Main loop

```rust
fn main() -> io::Result<()> {
    let _guard = RawGuard::new()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All), cursor::Hide)?;

    let map = [
        "########################################",
        "#......................................#",
        "#...####...............................#",
        "#...#..#...............................#",
        "#...####...............................#",
        "#......................................#",
        "#.................####.................#",
        "#.................#..#.................#",
        "#.................####.................#",
        "#......................................#",
        "########################################",
    ];
    let grid = Grid::from_str(&map);
    let mut player = (5, 3);

    loop {
        render(&grid, player, &mut stdout)?;
        if !handle_input(&grid, &mut player)? {
            break;
        }
    }

    Ok(())
}
```

Run. Move around with the arrows. Bump into a wall — player doesn't move. Press `q` — clean exit, terminal restored.

### One quirk: bottom-line prompt

After your program exits and raw mode is disabled, the shell prompt appears immediately below the last rendered row. Some people prefer to clear and reset position on exit. Add this in `RawGuard::drop`:

```rust
fn drop(&mut self) {
    let _ = execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show,
    );
    let _ = terminal::disable_raw_mode();
}
```

## Step 6 — Adding color

Crossterm supports colored output. Let's color walls cyan and the player yellow.

```rust
use crossterm::style::{Color, SetForegroundColor, ResetColor};

pub fn render(grid: &Grid, player: (usize, usize), stdout: &mut impl Write) -> io::Result<()> {
    use crossterm::{cursor, queue, style};

    queue!(stdout, cursor::MoveTo(0, 0))?;

    for y in 0..grid.height {
        for x in 0..grid.width {
            if (x, y) == player {
                queue!(
                    stdout,
                    SetForegroundColor(Color::Yellow),
                    style::Print('@'),
                    ResetColor,
                )?;
            } else {
                let c = grid.get(x, y);
                match c {
                    '#' => queue!(
                        stdout,
                        SetForegroundColor(Color::Cyan),
                        style::Print(c),
                        ResetColor,
                    )?,
                    _ => queue!(stdout, style::Print(c))?,
                }
            }
        }
        queue!(stdout, cursor::MoveToNextLine(1))?;
    }

    stdout.flush()?;
    Ok(())
}
```

Run. Walls cyan, player yellow. The moment the terminal feels like a game.

For complex styling you'd build a full palette type, but for today this is enough.

## Step 7 — Non-blocking input (preview)

`event::read()` blocks. That's fine when input drives the game (text adventure, chess). But for Snake, the world needs to advance whether you press a key or not. You want *non-blocking* input.

Use `event::poll(timeout)`:

```rust
use std::time::Duration;

if event::poll(Duration::from_millis(100))? {
    let ev = event::read()?;
    // handle
}
// otherwise, timeout expired: no key was pressed, do other work
```

`poll` returns `true` if an event is available. Then `read` is non-blocking. If `poll` returns `false`, no input came in within the timeout — your game-loop tick forward.

We'll use this extensively tomorrow.

## Common pitfalls

### Terminal stuck in raw mode

You ran your program, it crashed (panic, `todo!()`, whatever), and now your shell is broken — no echo, Ctrl-C doesn't work, backspace is garbage. Type `reset` (you can't see it) and press Enter. Shell restored.

To prevent: always use the RAII guard. `Drop` runs on unwinding panics.

### Writes don't appear

```rust
print!("hello");    // nothing shows up
```

In raw mode with the cursor hidden, `print!` isn't flushed eagerly. Always flush after writes you need to see now:

```rust
print!("hello");
io::stdout().flush()?;
```

Or use `execute!`/`queue!` which handle this for you.

### Flicker

You wrote `execute!` per-cell instead of accumulating with `queue!`. Each `execute!` call flushes — hundreds of syscalls per frame. Fix: build the frame with `queue!` and one `flush()` at the end. (Or assemble everything into a `String` and write it all at once.)

### `\n` doesn't return to column 0

In raw mode, `\n` moves down but not back to column 0. You need `\r\n` (or, better, `cursor::MoveToNextLine`). This catches people who were used to regular `println!`.

### `crossterm::event::read()` doesn't return

Probably your terminal doesn't have focus, or stdin is redirected. Make sure you're running interactively (not piped from a file).

### The mysterious key auto-repeat

Hold an arrow key and your player zooms. Or the OS sends weird rapid events. In terminal games this is usually fine; for tight control, you'd buffer input with a small deadzone. Out of scope today.

## What you learned

- Terminals are byte streams with escape-code commands.
- **Raw mode**: immediate key events, no buffering, no auto-echo.
- **`crossterm`** abstracts platform differences; `execute!`/`queue!` are the main tools.
- **Cursor control**: `MoveTo`, `Hide`, `Show`, `MoveToNextLine`.
- **RAII guards** for cleanup that runs on every exit path, including panics.
- **Color** with `SetForegroundColor`, `ResetColor`.
- **Blocking input** with `event::read`, preview of `event::poll` for games.

## Exercises

1. **Fog of war.** Only reveal cells within a radius of the player. Cells outside that radius render as space. Update on move.
2. **Mini-map.** In the bottom-right corner, render a smaller version of the whole grid. Use a different character (`.` floor, `@` player, etc.) and keep it updated.
3. **Status bar.** Below the grid, render a status line showing coordinates and steps taken. Use `Color::White` on `Color::Blue` background for readability.
4. **Resize handling.** Listen for `Event::Resize(w, h)` and redraw appropriately. Print an error if the terminal is smaller than the grid.
5. **Scrolling viewport.** Make the grid 100×50, but only render 40×15 around the player. The viewport scrolls as the player moves.

## What's next

Day 12 puts all this together into **Snake**. A proper game loop with fixed-timestep updates. Non-blocking input. Food and growth. Score tracking. Game-over screen. By the end you've got a real arcade game running in your terminal.

→ [Day 12 — Snake](day-12.md)
