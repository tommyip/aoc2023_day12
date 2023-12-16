# Advent of Code 2023 - Day 12

This is the optimized solution for [day 12] in Rust. The current runtime for
both parts on my M1 Pro is 600μs. For comparision, my unoptimized Python
solution runs in 180ms (300x speed up).

The main idea is to use bottom up dynamic programming and cache as much as
possible.

[day 12]: https://adventofcode.com/2023/day/12
