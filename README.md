# Hack Well

This is an experimental RE helper/modding framework for Animal Well. Currently it's only aimed at other people reverse engineering the game.

## Features

- Dumps all assets to `dumped_assets/`. Most assets will be dumped at startup, but encrypted assets will be dumped when accessed by the game.
- Replaces built-in game assets with files placed in the `modded_assets/` directory.

## Usage

Build with `cargo build`. (The binary will be in the `target/` directory.)

The compiled `hackwell.dll` can be used either by explicitly injecting it into the process at startup (using the `withdll` tool from Detours, for example), or by renaming it to `xinput9_1_0.dll` and placing it alongside `Animal Well.exe`.

## Contact

For any concerns, contact `@yuriks` on the datamining section of the Animal Well discord.
