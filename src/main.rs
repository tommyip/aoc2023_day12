use std::{env, fs, time::Instant};

use aoc2023_day12::{day12_parallel, day12_serial};

fn main() {
    let args = env::args().skip(1).take(2).collect::<Vec<_>>();
    let path = args.get(0).expect("Expected input file path, ie input.txt");
    let parallel = args.get(1).map_or(true, |flag| match flag.as_str() {
        "parallel" => true,
        "serial" => false,
        _ => panic!("Expected the optional second argument to either be `parallel` or `serial`"),
    });
    let input = fs::read(path).unwrap();

    let start = Instant::now();
    let (part1, part2) = if parallel {
        day12_parallel(&input)
    } else {
        day12_serial(&input)
    };
    let elapsed = start.elapsed().as_micros();

    println!("Part 1: {}", part1);
    println!("Part 2: {}", part2);
    println!("Elapsed {}us", elapsed);
}
