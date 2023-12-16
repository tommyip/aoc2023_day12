use rayon::prelude::*;
use std::{
    cell::RefCell,
    mem,
    ops::{Index, IndexMut},
    str,
};

use self::Record::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Record {
    #[allow(unused)]
    Operational = b'.',
    Damaged = b'#',
    Unknown = b'?',
}

type UGroup = u8;

#[derive(Debug)]
struct DP<'a> {
    n_records: usize,
    n_groups: usize,
    values: &'a mut Vec<u64>,
}

pub struct Row<'a> {
    records: &'a [Record],
    groups: &'a [UGroup],
    repeated_records: &'a [Record],
    repeated_groups: &'a [UGroup],
}

/// Solve Day 12 using bottom up dynamic programming
fn solve(records: &[Record], groups: &[UGroup], dp_buf: &mut Vec<u64>) -> u64 {
    let nr = records.len();
    let ng = groups.len();

    let mut dp = DP::new(nr, ng, dp_buf);

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

    // Pre-calculate the maximum number of consecutively damaged or
    // unknown (to be set as damaged) springs reachable from each record.
    let mut damage_count = 0;
    for (i, lookahead) in dp.damage_lookaheads_mut().into_iter().enumerate().rev() {
        match records[i] {
            Damaged | Unknown => damage_count += 1,
            Operational => damage_count = 0,
        }
        *lookahead = damage_count;
    }

    for gi in (0..ng).rev() {
        for ri in (0..nr).rev() {
            dp[(gi, ri)] = match records[ri] {
                // Already commited to `.`, same arrangements as tail
                Operational => dp[(gi, ri + 1)],
                Damaged | Unknown => {
                    let group_len = groups[gi] as usize;
                    // Try committing group to all `#`s.
                    // This is possible if the next `group_len` records are all `#` or `?` and the record
                    // after the group is either a `.`, `?` or EOF.
                    let damaged_arragements = if group_len as u64 <= dp.damage_lookaheads()[ri]
                        && (ri + group_len >= records.len() || records[ri + group_len] != Damaged)
                    {
                        dp[(gi + 1, ri + group_len + 1)]
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

pub fn parse_lines<'a>(input: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    input.strip_suffix(&[b'\n']).unwrap().split(|&byte| byte == b'\n')
}

pub fn day12_parallel(input: &[u8]) -> (u64, u64) {
    // Reuse allocations
    thread_local! {
        static DP: RefCell<Vec<u64>> = RefCell::new(vec![]);
        static REPEATED_RECORDS: RefCell<Vec<Record>> = RefCell::new(vec![]);
        static REPEATED_GROUPS: RefCell<Vec<UGroup>> = RefCell::new(vec![]);
    }
    parse_lines(&input)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|line| {
            DP.with_borrow_mut(|dp| {
                REPEATED_RECORDS.with_borrow_mut(|repeated_records| {
                    REPEATED_GROUPS.with_borrow_mut(|repeated_groups| {
                        let row = Row::parse(line, repeated_records, repeated_groups);
                        let part1 = solve(row.records, row.groups, dp);
                        let part2 = solve(row.repeated_records, row.repeated_groups, dp);
                        (part1, part2)
                    })
                })
            })
        })
        .reduce(|| (0, 0), |(acc_p1, acc_p2), (p1, p2)| (acc_p1 + p1, acc_p2 + p2))
}

pub fn day12_serial(input: &[u8]) -> (u64, u64) {
    let mut dp = vec![];
    let mut records_buf = vec![];
    let mut groups_buf = vec![];
    let mut part1 = 0;
    let mut part2 = 0;
    for line in parse_lines(&input) {
        let row = Row::parse(line, &mut records_buf, &mut groups_buf);
        part1 += solve(row.records, row.groups, &mut dp);
        part2 += solve(row.repeated_records, row.repeated_groups, &mut dp);
    }
    (part1, part2)
}

impl<'a> Row<'a> {
    pub fn parse(line: &'a [u8], repeated_records: &'a mut Vec<Record>, repeated_groups: &'a mut Vec<UGroup>) -> Self {
        let space_idx = line.iter().rposition(|&c| c == b' ').unwrap();
        let records: &[Record] = unsafe { mem::transmute(&line[..space_idx]) };

        let chunk_len = records.len() + 1;
        repeated_records.resize(chunk_len * 5 - 1, Unknown);
        for i in 0..5 {
            repeated_records[chunk_len * i..chunk_len * i + records.len()].copy_from_slice(records);
            if i != 4 {
                repeated_records[chunk_len * i + records.len()] = Unknown;
            }
        }

        let groups = line[space_idx + 1..]
            .split(|&c| c == b',')
            .map(|digits| unsafe { str::from_utf8_unchecked(digits) }.parse::<UGroup>().unwrap());
        repeated_groups.clear();
        repeated_groups.extend(groups);
        let n_groups = repeated_groups.len();

        repeated_groups.resize(n_groups * 5, 0);
        for i in 1..5 {
            repeated_groups.copy_within(..n_groups, i * n_groups);
        }

        Self {
            records: &repeated_records[..records.len()],
            groups: &repeated_groups[..n_groups],
            repeated_records: &repeated_records[..],
            repeated_groups: &repeated_groups[..],
        }
    }
}

impl<'a> DP<'a> {
    /// DP arr is not zero-ed out! Make sure cells are written before read.
    fn new(n_records: usize, n_groups: usize, buf: &'a mut Vec<u64>) -> Self {
        let n_damage_lookaheads = n_records;
        let n_records = n_records + 1;
        let n_groups = n_groups + 1;
        // Add an additional n_records to store the damage lookahead cache
        buf.resize(n_records * n_groups + n_damage_lookaheads, 0);
        Self {
            n_records,
            n_groups,
            values: buf,
        }
    }

    fn damage_lookaheads(&self) -> &[u64] {
        &self.values[self.n_records * self.n_groups..]
    }

    fn damage_lookaheads_mut(&mut self) -> &mut [u64] {
        &mut self.values[self.n_records * self.n_groups..]
    }
}

impl Index<(usize, usize)> for DP<'_> {
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

impl IndexMut<(usize, usize)> for DP<'_> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (group_idx, record_idx) = index;
        &mut self.values[self.n_records * group_idx + record_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve_one(input: &str) -> u64 {
        let mut dp_buf = vec![];
        let mut repeated_records = vec![];
        let mut repeated_groups = vec![];
        let Row { records, groups, .. } = Row::parse(input.as_bytes(), &mut repeated_records, &mut repeated_groups);
        solve(records, &groups, &mut dp_buf)
    }

    fn solve_two(input: &str) -> u64 {
        let mut dp_buf = vec![];
        let mut repeated_records = vec![];
        let mut repeated_groups = vec![];
        let Row {
            repeated_records,
            repeated_groups,
            ..
        } = Row::parse(input.as_bytes(), &mut repeated_records, &mut repeated_groups);
        solve(repeated_records, &repeated_groups, &mut dp_buf)
    }

    #[test]
    fn test_part1() {
        assert_eq!(1, solve_one("???.### 1,1,3"));
        assert_eq!(4, solve_one(".??..??...?##. 1,1,3"));
        assert_eq!(1, solve_one("?#?#?#?#?#?#?#? 1,3,1,6"));
        assert_eq!(1, solve_one("????.#...#... 4,1,1"));
        assert_eq!(4, solve_one("????.######..#####. 1,6,5"));
        assert_eq!(10, solve_one("?###???????? 3,2,1"));
    }

    #[test]
    fn test_part2() {
        assert_eq!(1, solve_two("???.### 1,1,3"));
        assert_eq!(16384, solve_two(".??..??...?##. 1,1,3"));
        assert_eq!(1, solve_two("?#?#?#?#?#?#?#? 1,3,1,6"));
        assert_eq!(16, solve_two("????.#...#... 4,1,1"));
        assert_eq!(2500, solve_two("????.######..#####. 1,6,5"));
        assert_eq!(506250, solve_two("?###???????? 3,2,1"));
    }
}
