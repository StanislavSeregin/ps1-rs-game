# ps1-game

A `no_std` Rust homebrew experiment for the PlayStation 1 (PS1/PSX), targeting the MIPS R3000A bare metal.

I always wanted to make something for my favorite childhood console — and now I finally can.

## What's Inside

- **Custom audio engine** — tracker-style SPU driver with pattern-based playback. The demo ships with a working soundtrack.
- **Cooperative multitasking runtime** — lets audio, gameplay logic, and rendering coexist as independent tasks that share the single CPU without stepping on each other.

These are not part of the underlying SDK — both the task system and the audio engine were built from scratch for this project.

## Getting Started

The project includes a dev container — open it and everything is ready to go.

### Build

```sh
cargo psx build --toolchain nightly-2025-05-23
```

## Acknowledgements

Huge thanks to [@ayrtonm](https://github.com/ayrtonm) for creating [psx-sdk-rs](https://github.com/ayrtonm/psx-sdk-rs) — the Rust SDK for PlayStation 1 that made this project possible.
