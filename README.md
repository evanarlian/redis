[![progress-banner](https://backend.codecrafters.io/progress/redis/7f86266e-15a5-4bd5-b6f1-a38578efc151)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

This is a starting point for Rust solutions to the
["Build Your Own Redis" Challenge](https://codecrafters.io/challenges/redis).

In this challenge, you'll build a toy Redis clone that's capable of handling
basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
event loops, the Redis protocol and more.

**Note**: If you're viewing this repo on GitHub, head over to
[codecrafters.io](https://codecrafters.io) to try the challenge.


# Features
* Common and simple redis commands.
* Can be used with standard redis client.
* Efficient data structure for O(1) get, set, random deletion (active key eviction)
* Restore key-values from RDB file.

# Flaws
* No aggregate data structures such as list, set, etc.
* One connection is mapped to one thread, thus cannot handle massive number of connections.
* RwLock is kinda pointless, since seemingly read-only operations such as GET can be a write operation too because of passive key eviction.
* RDB reader is not implemented completely.
