# discord-ext-songbird

Yet another `discord.VoiceProtocol` implementation using [`songbird`] as the backing
library.

The client imitates discord.py's standard [`VoiceClient`], with a few extra features
like volume and seeking built-in.

Requires Python 3.8 or newer. The minimum Python version will bump as discord.py bumps
its minimum Python version.

[`songbird`]: https://docs.rs/songbird/latest/songbird/
[`VoiceClient`]: https://discordpy.readthedocs.io/en/latest/api.html#discord.VoiceClient

## Installation

This package is not available on PyPI due to a name conflict. Instead, it is published
on a local index:

```
pip install --index-url https://beer-psi.github.io/discord-ext-songbird discord-ext-songbird
```

## Building

A working Rust compiler with `rustc` and `cargo` is required to build this package.
The minimum supported Rust version is Rust 1.83.

## Development

This project uses `uv` and `maturin`. `maturin` is part of the development dependencies.

```sh
uv sync --all-groups
source .venv/bin/activate
maturin develop
```
