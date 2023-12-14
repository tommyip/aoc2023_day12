use std::{
    env, fs, mem,
    ops::{BitAnd, Index, IndexMut},
    time::Instant,
};

use self::Record::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Record {
    Operational = b'.',
    Damaged = b'#',
    Unknown = b'?',
}

// Store spring groups directly on the stack
// The max number of groups is 6 from the actual input
stack_vec::stack!(pub type StackVec6 StackVec6IntoIter 6);
type UGroup = u8;
type Groups = StackVec6<UGroup>;

/// Look Ma, Zero Copy!
///
/// Like a `&[Record]` but can be repeated and indexed
/// with no runtime overhead.
struct RepeatedRecords<'a, const N: usize>(&'a [Record]);
struct RepeatedGroups<'a, const N: usize>(&'a [UGroup]);

#[derive(Debug)]
struct DP {
    n_records: usize,
    n_groups: usize,
    values: Vec<u64>,
}

struct Row<'a> {
    records: &'a [Record],
    groups: Groups,
}

fn solve<const N: usize>(row: &Row<'_>) -> u64 {
    let records = RepeatedRecords::<'_, N>(row.records);
    let groups = RepeatedGroups::<'_, N>(&row.groups);
    let nr = records.len();
    let ng = groups.len();

    let mut dp = DP::new(nr + 1, ng + 1);

    // Base cases

    // 1. No # groups left
    dp[(ng, nr)] = 1; // No records left either
    for i in (0..nr).rev() {
        // 1 arragement if all trailing records are not #s
        dp[(ng, i)] = (records[i] != Damaged) as u64 & dp[(ng, i + 1)];
    }

    // 2. No records left but some groups left
    for i in 0..ng {
        dp[(i, nr)] = 0;
    }

    for gi in (0..ng).rev() {
        for ri in (0..nr).rev() {
            dp[(gi, ri)] = match records[ri] {
                // Already commited to `.`, same arrangements as tail
                Operational => dp[(gi, ri + 1)],
                Damaged | Unknown => {
                    let group_len = groups[gi] as usize;
                    // Try committing group to all `#`
                    let damaged_arragements = if ri + group_len <= nr {
                        let can_commit = (ri..ri + group_len)
                            .map(|i| records[i])
                            .all(|r| r == Damaged || r == Unknown)
                            .bitand(
                                ri + group_len == nr
                                    || records[ri + group_len] == Operational
                                    || records[ri + group_len] == Unknown,
                            );
                        if can_commit {
                            // Possible, same arragemnts as tail and next group
                            dp[(gi + 1, ri + group_len + 1)]
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    if records[ri] == Unknown {
                        // Also try commtting to `.`
                        damaged_arragements + dp[(gi, ri + 1)]
                    } else {
                        damaged_arragements
                    }
                }
            };
        }
    }
    return dp[(0, 0)];
}

fn parse<'a>(input: &'a [u8]) -> impl Iterator<Item = Row<'a>> {
    input
        .strip_suffix(&[b'\n'])
        .unwrap()
        .split(|&byte| byte == b'\n')
        .map(|line| {
            let space_idx = line.iter().rposition(|&c| c == b' ').unwrap();
            let records: &[Record] = unsafe { mem::transmute(&line[..space_idx]) };
            let groups = line[space_idx + 1..]
                .split(|&c| c == b',')
                .map(|digits| {
                    (if digits.len() == 1 {
                        digits[0] - b'0'
                    } else {
                        (digits[0] - b'0') * 10 + digits[1] - b'0'
                    })
                    .into()
                })
                .collect();
            Row { records, groups }
        })
}

impl<'a, const N: usize> RepeatedRecords<'a, N> {
    fn len(&self) -> usize {
        (self.0.len() + 1) * N - 1
    }
}

impl<'a, const N: usize> Index<usize> for RepeatedRecords<'a, N> {
    type Output = Record;

    /// This returns `?` when indexing the record at repeated_record.len() + 1
    /// instead of panicking for out of bound access. This is wrong but it
    /// is also unreachable. 🤷
    fn index(&self, index: usize) -> &Self::Output {
        debug_assert_ne!((self.0.len() + 1) * N, index);
        let i = index % (self.0.len() + 1);
        if i < self.0.len() {
            &self.0[i]
        } else {
            &Unknown // delimiter
        }
    }
}

impl<'a, const N: usize> RepeatedGroups<'a, N> {
    fn len(&self) -> usize {
        self.0.len() * N
    }
}

impl<'a, const N: usize> Index<usize> for RepeatedGroups<'a, N> {
    type Output = UGroup;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index % self.0.len()]
    }
}

impl DP {
    fn new(n_records: usize, n_groups: usize) -> Self {
        Self {
            n_records,
            n_groups,
            values: vec![0; n_records * n_groups],
        }
    }
}

impl Index<(usize, usize)> for DP {
    type Output = u64;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (group_idx, record_idx) = index;
        if record_idx < self.n_records {
            &self.values[self.n_records * group_idx + record_idx]
        } else if group_idx == self.n_groups - 1 {
            &1
        } else {
            &0
        }
    }
}

impl IndexMut<(usize, usize)> for DP {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (group_idx, record_idx) = index;
        &mut self.values[self.n_records * group_idx + record_idx]
    }
}

fn main() {
    debug_assert_eq!(
        mem::size_of::<&[u8]>(),
        mem::size_of::<RepeatedRecords<'_, 5>>()
    );

    let path = env::args()
        .skip(1)
        .next()
        .expect("Expected input file path, ie input.txt");
    let input = fs::read(path).unwrap();
    let start = Instant::now();

    let mut part1 = 0;
    let mut part2 = 0;
    for row in parse(&input) {
        part1 += solve::<1>(&row);
        part2 += solve::<5>(&row);
    }

    let elapsed = start.elapsed().as_micros();

    println!("Part 1: {}", part1);
    println!("Part 2: {}", part2);
    println!("Elapsed {}us", elapsed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(input: &str) -> Row<'_> {
        parse(input.as_bytes()).next().unwrap()
    }

    #[test]
    fn test_part1() {
        assert_eq!(1, solve::<1>(&parse_one("???.### 1,1,3\n")));
        assert_eq!(4, solve::<1>(&parse_one(".??..??...?##. 1,1,3\n")));
        assert_eq!(1, solve::<1>(&parse_one("?#?#?#?#?#?#?#? 1,3,1,6\n")));
        assert_eq!(1, solve::<1>(&parse_one("????.#...#... 4,1,1\n")));
        assert_eq!(4, solve::<1>(&parse_one("????.######..#####. 1,6,5\n")));
        assert_eq!(10, solve::<1>(&parse_one("?###???????? 3,2,1\n")));
    }

    #[test]
    fn test_part2() {
        assert_eq!(1, solve::<5>(&parse_one("???.### 1,1,3\n")));
        assert_eq!(16384, solve::<5>(&parse_one(".??..??...?##. 1,1,3\n")));
        assert_eq!(1, solve::<5>(&parse_one("?#?#?#?#?#?#?#? 1,3,1,6\n")));
        assert_eq!(16, solve::<5>(&parse_one("????.#...#... 4,1,1\n")));
        assert_eq!(2500, solve::<5>(&parse_one("????.######..#####. 1,6,5\n")));
        assert_eq!(506250, solve::<5>(&parse_one("?###???????? 3,2,1\n")));
    }
}
