use std::{array, collections::HashSet, iter};

pub fn start() {
    println!("Queens:\n",);
    let res = with_benchset::eight_queens_problem();
    BoardIdx::display_board(res);
}

/// A subset of the 8x8 chess board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BoardSet {
    /// A flattened bit-set of the board values
    inner: u64,
}
impl BoardSet {
    const fn none() -> Self {
        Self { inner: 0 }
    }

    const fn all() -> Self {
        const { Self::none().invert() }
    }

    #[must_use]
    const fn invert(self) -> Self {
        Self { inner: !self.inner }
    }

    #[inline(always)]
    const fn board_idx_to_idx(board_idx: BoardIdx) -> u8 {
        board_idx.col * 8 + board_idx.row
    }

    #[inline(always)]
    const fn idx_to_board_idx(idx: u8) -> BoardIdx {
        BoardIdx {
            col: idx / 8,
            row: idx % 8,
        }
    }

    #[inline(always)]
    const fn set_idx(&mut self, idx: u8, to: bool) {
        // This gets optimally optimized when `to` is known
        let mask = !(1 << idx);
        let flag = (to as u64) << idx;
        self.inner = (self.inner & mask) | flag;
    }

    #[inline(always)]
    const fn set(&mut self, board_idx: BoardIdx, to: bool) {
        self.set_idx(Self::board_idx_to_idx(board_idx), to)
    }

    #[inline(always)]
    const fn remove(&mut self, board_idx: BoardIdx) {
        self.set(board_idx, false);
    }

    #[inline(always)]
    const fn contains_idx(&self, idx: u8) -> bool {
        (self.inner >> idx) & 1 == 1
    }

    fn iter(self) -> impl IntoIterator<Item = BoardIdx> {
        (0u8..64).filter_map(move |idx| {
            self.contains_idx(idx)
                .then_some(Self::idx_to_board_idx(idx))
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoardIdx {
    col: u8,
    row: u8,
}

impl BoardIdx {
    pub fn display_board(on: impl IntoIterator<Item = BoardIdx>) {
        let active = on.into_iter().collect::<HashSet<BoardIdx>>();
        for col in 0..8 {
            for row in 0..8 {
                if active.contains(&BoardIdx { col, row }) {
                    print!("# ");
                } else {
                    print!("- ")
                }
            }
            println!();
        }
    }

    fn is_valid(self) -> bool {
        self.col < 8 && self.row < 8
    }

    fn iter_col(self) -> [BoardIdx; 8] {
        array::from_fn(|row| BoardIdx {
            col: self.col,
            row: row as u8,
        })
    }

    fn iter_row(self) -> [BoardIdx; 8] {
        array::from_fn(|col| BoardIdx {
            col: col as u8,
            row: self.row,
        })
    }

    fn iter_diagonals(self) -> impl IntoIterator<Item = BoardIdx> {
        let mut top_left = {
            let advance_by = self.col.min(self.row);
            BoardIdx {
                col: self.col - advance_by,
                row: self.row - advance_by,
            }
        };
        let top_left_iter = iter::from_fn(move || {
            top_left.is_valid().then(|| {
                let tl = top_left;
                top_left.col += 1;
                top_left.row += 1;
                tl
            })
        });

        let mut top_right = {
            let advance_by = (8 - self.col).min(self.row);
            BoardIdx {
                col: self.col + advance_by,
                row: self.row - advance_by,
            }
        };
        let top_right_iter = iter::from_fn(move || {
            top_right.is_valid().then(|| {
                let tl = top_right;
                top_right.col = top_right.col.wrapping_sub(1);
                top_right.row += 1;
                tl
            })
        });

        top_left_iter.chain(top_right_iter)
    }
}

pub mod with_benchset {
    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res = eight_queens_problem_inner(BoardSet::all(), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: BoardSet,
        queens_remaining: usize,
    ) -> Option<Vec<BoardIdx>> {
        if queens_remaining == 0 {
            return Some(vec![]);
        }

        for to_place in available.iter() {
            let mut new_available = available.clone();
            for i in to_place
                .iter_col()
                .into_iter()
                .chain(to_place.iter_row())
                .chain(to_place.iter_diagonals())
            {
                new_available.remove(i);
            }

            if let Some(mut solution) =
                eight_queens_problem_inner(new_available, queens_remaining - 1)
            {
                if solution.is_empty() {
                    solution.reserve_exact(8);
                }
                assert!(solution.capacity() == 8);
                assert!(solution.len() < 8);
                solution.push(to_place);
                return Some(solution);
            }
        }

        None
    }
}

pub mod with_benchset_unsafe_opts {
    use std::{hint::assert_unchecked, mem::MaybeUninit};

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let mut res = [MaybeUninit::uninit(); 8];
        eight_queens_problem_inner(BoardSet::all(), 8, &mut res).unwrap();
        std::array::from_fn(|i| unsafe { res[i].assume_init() })
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: BoardSet,
        queens_remaining: usize,
        solution: &mut [MaybeUninit<BoardIdx>; 8],
    ) -> Option<()> {
        if queens_remaining == 0 {
            return Some(());
        }

        for to_place in available.iter() {
            let mut new_available = available.clone();
            for i in to_place
                .iter_col()
                .into_iter()
                .chain(to_place.iter_row())
                .chain(to_place.iter_diagonals())
            {
                new_available.remove(i);
            }

            if let Some(()) =
                eight_queens_problem_inner(new_available, queens_remaining - 1, solution)
            {
                unsafe {
                    solution
                        .get_unchecked_mut(queens_remaining - 1)
                        .write(to_place);
                }
                return Some(());
            }
        }

        None
    }
}

pub mod with_benchset_tinyvec {
    use tinyvec::ArrayVec;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res = eight_queens_problem_inner(BoardSet::all(), 8).unwrap();
        *res.as_inner()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: BoardSet,
        queens_remaining: usize,
    ) -> Option<ArrayVec<[BoardIdx; 8]>> {
        if queens_remaining == 0 {
            return Some(ArrayVec::new());
        }

        for to_place in available.iter() {
            let mut new_available = available.clone();
            for i in to_place
                .iter_col()
                .into_iter()
                .chain(to_place.iter_row())
                .chain(to_place.iter_diagonals())
            {
                new_available.remove(i);
            }

            if let Some(mut solution) =
                eight_queens_problem_inner(new_available, queens_remaining - 1)
            {
                solution.push(to_place);
                return Some(solution);
            }
        }

        None
    }
}

pub mod with_tinyset {
    use tinyset::SetU32;

    use crate::eight_queens::{BoardIdx, BoardSet};

    fn u32_to_idx(n: u32) -> BoardIdx {
        let [col, row, _, _] = n.to_ne_bytes();
        BoardIdx { col, row }
    }

    fn idx_to_u32(idx: BoardIdx) -> u32 {
        u32::from_ne_bytes([idx.col, idx.row, 0, 0])
    }

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res = eight_queens_problem_inner(
            &SetU32::from_iter(BoardSet::all().iter().into_iter().map(idx_to_u32)),
            8,
        )
        .unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &SetU32,
        queens_remaining: usize,
    ) -> Option<Vec<BoardIdx>> {
        if queens_remaining == 0 {
            return Some(vec![]);
        }

        for to_place in available.iter() {
            let to_place = u32_to_idx(to_place);

            let mut new_available = available.clone();
            for i in to_place
                .iter_col()
                .into_iter()
                .chain(to_place.iter_row())
                .chain(to_place.iter_diagonals())
            {
                new_available.remove(idx_to_u32(i));
            }

            if let Some(mut solution) =
                eight_queens_problem_inner(&new_available, queens_remaining - 1)
            {
                solution.push(to_place);
                return Some(solution);
            }
        }

        None
    }
}

pub mod with_hashset {
    use std::collections::HashSet;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res =
            eight_queens_problem_inner(&HashSet::from_iter(BoardSet::all().iter()), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &HashSet<BoardIdx>,
        queens_remaining: usize,
    ) -> Option<Vec<BoardIdx>> {
        if queens_remaining == 0 {
            return Some(vec![]);
        }

        for to_place in available.iter().copied() {
            let mut new_available = available.clone();
            for i in to_place
                .iter_col()
                .into_iter()
                .chain(to_place.iter_row())
                .chain(to_place.iter_diagonals())
            {
                new_available.remove(&i);
            }

            if let Some(mut solution) =
                eight_queens_problem_inner(&new_available, queens_remaining - 1)
            {
                solution.push(to_place);
                return Some(solution);
            }
        }

        None
    }
}
