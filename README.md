# Advent of Code 2023 - Day 12

This is my optimized solution for [day 12] in Rust. The current runtime for
both parts on my Macbook Pro 2021 (M1 Pro) is 570μs. For comparision, my
unoptimized [Python solution] runs in 180ms (315x speed up).

The main idea is to use bottom up dynamic programming and cache as much as
possible.

[day 12]: https://adventofcode.com/2023/day/12
[Python solution]: https://github.com/tommyip/aoc2023/blob/master/python/day12.py
