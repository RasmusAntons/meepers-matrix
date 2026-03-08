# meepers-matrix

A WIP matrix bot using the [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk).

For now, I am trying to port some of the features of my [notpron-discord-bot](https://github.com/RasmusAntons/notpron-discord-bot) to matrix.
I am also using this as an opportunity to learn Rust, so don't necessarily expect reasonable code.

## Setup

The only parameters are the environment variables `BOT_USER_ID` and `BOT_PASSWORD`. 

Example usage:
```bash
BOT_USER_ID="@meepers:enigmatics.org" BOT_PASSWORD="<password>" cargo run
```
