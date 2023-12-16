use rayon::prelude::*;
use stack_vec::StackVec8;
use std::{
    cell::RefCell,
    mem,
    ops::{Index, IndexMut},
    str,
};

use self::Record::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Record {
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

struct Row<'a> {
    records: &'a [Record],
    groups: StackVec8<UGroup>,
}

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
                .map(|digits| unsafe { str::from_utf8_unchecked(digits) }.parse::<UGroup>().unwrap())
                .collect::<StackVec8<_>>();
            Row { records, groups }
        })
}

pub fn day12_parallel(input: &[u8]) -> (u64, u64) {
    // Reuse allocations
    thread_local! {
        static DP: RefCell<Vec<u64>> = RefCell::new(vec![]);
        static REPEATED_RECORDS: RefCell<Vec<Record>> = RefCell::new(vec![]);
        static REPEATED_GROUPS: RefCell<Vec<UGroup>> = RefCell::new(vec![]);
    }
    parse(&input)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|row| {
            DP.with_borrow_mut(|dp| {
                let (part1_records, part1_groups) = row.part1();
                let part1 = solve(part1_records, &part1_groups, dp);

                let part2 = REPEATED_RECORDS.with_borrow_mut(|repeated_records| {
                    REPEATED_GROUPS.with_borrow_mut(|repeated_groups| {
                        let (part2_records, part2_groups) = row.part2(repeated_records, repeated_groups);
                        solve(&part2_records, &part2_groups, dp)
                    })
                });
                (part1, part2)
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
    for row in parse(&input) {
        let (part1_records, part1_groups) = row.part1();
        part1 += solve(part1_records, &part1_groups, &mut dp);
        let (part2_records, part2_groups) = row.part2(&mut records_buf, &mut groups_buf);
        part2 += solve(&part2_records, &part2_groups, &mut dp)
    }
    (part1, part2)
}

impl<'a> Row<'a> {
    fn part1(&self) -> (&[Record], &[UGroup]) {
        (&self.records, &self.groups)
    }

    fn part2<'b>(
        &self,
        records_buf: &'b mut Vec<Record>,
        groups_buf: &'b mut Vec<UGroup>,
    ) -> (&'b [Record], &'b [UGroup]) {
        let chunk_len = self.records.len() + 1;
        records_buf.resize(chunk_len * 5 - 1, Unknown);
        for i in 0..5 {
            records_buf[chunk_len * i..chunk_len * i + self.records.len()].copy_from_slice(self.records);
            if i != 4 {
                records_buf[chunk_len * i + self.records.len()] = Unknown;
            }
        }
        groups_buf.resize(self.groups.len() * 5, 0);
        for i in 0..5 {
            groups_buf[self.groups.len() * i..self.groups.len() * i + self.groups.len()].copy_from_slice(&self.groups);
        }

        (records_buf, groups_buf)
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
        let row = parse(input.as_bytes()).next().unwrap();
        let (records, groups) = row.part1();
        solve(records, &groups, &mut dp_buf)
    }

    fn solve_two(input: &str) -> u64 {
        let mut dp_buf = vec![];
        let mut repeated_records = vec![];
        let mut repeated_groups = vec![];
        let row = parse(input.as_bytes()).next().unwrap();
        let (records, groups) = row.part2(&mut repeated_records, &mut repeated_groups);
        solve(&records, &groups, &mut dp_buf)
    }

    #[test]
    fn test_part1() {
        assert_eq!(1, solve_one("???.### 1,1,3\n"));
        assert_eq!(4, solve_one(".??..??...?##. 1,1,3\n"));
        assert_eq!(1, solve_one("?#?#?#?#?#?#?#? 1,3,1,6\n"));
        assert_eq!(1, solve_one("????.#...#... 4,1,1\n"));
        assert_eq!(4, solve_one("????.######..#####. 1,6,5\n"));
        assert_eq!(10, solve_one("?###???????? 3,2,1\n"));
    }

    #[test]
    fn test_part2() {
        assert_eq!(1, solve_two("???.### 1,1,3\n"));
        assert_eq!(16384, solve_two(".??..??...?##. 1,1,3\n"));
        assert_eq!(1, solve_two("?#?#?#?#?#?#?#? 1,3,1,6\n"));
        assert_eq!(16, solve_two("????.#...#... 4,1,1\n"));
        assert_eq!(2500, solve_two("????.######..#####. 1,6,5\n"));
        assert_eq!(506250, solve_two("?###???????? 3,2,1\n"));
    }
}
