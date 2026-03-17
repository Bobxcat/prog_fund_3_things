use std::{
    array,
    collections::HashSet,
    hash::{BuildHasher, Hasher},
    hint::{assert_unchecked, unreachable_unchecked},
    iter,
    ops::BitAnd,
};

#[macro_export]
macro_rules! use_with_all {
    (with $name:ident; $($expr:tt)*) => {
        {
            #[allow(unused)] use $crate::eight_queens::$name as imp;
            #[allow(unused)] const IMP_NAME: &'static str = stringify!($name);
            $($expr)*
        }
    };
    ($($expr:tt)*) => {{
        use_with_all!(with with_boardset; $($expr)*);
        use_with_all!(with with_boardset_bitwise_remove_col_row; $($expr)*);
        use_with_all!(with with_boardset_unsafe_opts; $($expr)*);
        use_with_all!(with with_boardset_tinyvec; $($expr)*);
        use_with_all!(with with_tinyset; $($expr)*);
        use_with_all!(with with_hashset; $($expr)*);
        use_with_all!(with with_hashset_cached_allocs; $($expr)*);
        use_with_all!(with with_fxhashset; $($expr)*);
        use_with_all!(with with_specialhashset; $($expr)*);
        use_with_all!(with with_btreeset; $($expr)*);
        use_with_all!(with with_vec; $($expr)*);
        use_with_all!(with with_iter; $($expr)*);
        use_with_all!(with with_iter_boardset; $($expr)*);
        use_with_all!(with with_iter_boardset_cursor; $($expr)*);
        use_with_all!(with with_iter_boardset_cursor_laneopts; $($expr)*);
    }};
}

pub fn start() {
    // _display_removals();

    println!("Queens:\n",);
    // let res = with_boardset::eight_queens_problem();
    // let res = with_iter_boardset::eight_queens_problem();
    let res = with_iter_boardset_cursor::eight_queens_problem();
    BoardIdx::display_board(res);
}

fn _display_removals() {
    for col in 0..8 {
        let mut board = BoardSet::all();
        board.remove_col(col);
        println!("col{col}:");
        BoardIdx::display_board(board.iter());
        println!();
    }

    for row in 0..8 {
        let mut board = BoardSet::all();
        board.remove_row(row);
        println!("row{row}:");
        BoardIdx::display_board(board.iter());
        println!();
    }

    for col in 0..8 {
        for row in 0..8 {
            let mut board = BoardSet::all();
            BoardIdx { col, row }
                .iter_diagonals()
                .into_iter()
                .for_each(|i| board.remove(i));

            println!("diag({col},{row}):");
            BoardIdx::display_board(board.iter());
            println!();
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SpecialHashBuildHasher;
impl BuildHasher for SpecialHashBuildHasher {
    type Hasher = SpecialHasher;

    #[inline(always)]
    fn build_hasher(&self) -> Self::Hasher {
        SpecialHasher(0)
    }
}

struct SpecialHasher(u64);
impl Hasher for SpecialHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        let &[col, row] = bytes else {
            unreachable!("Should only be called by BoardIdx::hash")
        };
        self.0 = (col * 8 + row) as u64;
    }
}

/// A subset of the 8x8 chess board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BoardSet {
    /// A flattened bit-set of the board values
    ///
    /// The flattening scheme is in row-major order
    inner: u64,
}

impl Default for BoardSet {
    fn default() -> Self {
        Self::none()
    }
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
    const fn remove_col(&mut self, col: u8) {
        assert!(col < 8);
        let mask: u64 = 0xff; // The first 8 bits
        self.inner &= !(mask << col * 8);
    }

    #[inline(always)]
    const fn remove_row(&mut self, row: u8) {
        assert!(row < 8);
        let mask: u64 = 0x101010101010101; // Every 8th bit, starting at bit 0
        self.inner &= !(mask << row);
    }

    #[inline(always)]
    const fn contains_idx(&self, idx: u8) -> bool {
        (self.inner >> idx) & 1 == 1
    }

    fn iter(self) -> impl IntoIterator<Item = BoardIdx> {
        (0u8..64)
            .filter_map(move |idx| {
                self.contains_idx(idx)
                    .then_some(Self::idx_to_board_idx(idx))
            })
            .inspect(|i| {
                // SAFETY: We're being safe lol
                unsafe {
                    assert_unchecked(i.col < 8);
                    assert_unchecked(i.row < 8);
                }
            })
    }
}

impl BitAnd for BoardSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            inner: self.inner & rhs.inner,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoardIdx {
    col: u8,
    row: u8,
}

impl std::hash::Hash for BoardIdx {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&[self.col, self.row]);
    }
}

impl BoardIdx {
    pub fn display_board(on: impl IntoIterator<Item = BoardIdx>) {
        let active = on.into_iter().collect::<HashSet<BoardIdx>>();
        for row in 0..8 {
            for col in 0..8 {
                if active.contains(&BoardIdx { col, row }) {
                    print!("# ");
                } else {
                    print!("- ")
                }
            }
            println!();
        }
    }

    const fn is_valid(self) -> bool {
        self.col < 8 && self.row < 8
    }

    #[inline]
    fn iter_col(self) -> [BoardIdx; 8] {
        array::from_fn(|row| BoardIdx {
            col: self.col,
            row: row as u8,
        })
    }

    #[inline]
    fn iter_row(self) -> [BoardIdx; 8] {
        array::from_fn(|col| BoardIdx {
            col: col as u8,
            row: self.row,
        })
    }

    #[inline]
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
            let advance_by = (7 - self.col).min(self.row);
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

pub mod with_boardset_bitwise_remove_col_row {
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
            new_available.remove_col(to_place.col);
            new_available.remove_row(to_place.row);
            to_place
                .iter_diagonals()
                .into_iter()
                .for_each(|i| new_available.remove(i));

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

pub mod with_boardset {
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
            to_place
                .iter_diagonals()
                .into_iter()
                .chain(to_place.iter_col())
                .chain(to_place.iter_row())
                .for_each(|i| new_available.remove(i));

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

pub mod with_boardset_unsafe_opts {
    use std::mem::MaybeUninit;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res = eight_queens_problem_inner(BoardSet::all(), 0).unwrap();
        std::array::from_fn(|i| unsafe { res[i].assume_init() })
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: BoardSet,
        idx: usize,
    ) -> Option<[MaybeUninit<BoardIdx>; 8]> {
        if idx >= 8 {
            return Some([MaybeUninit::uninit(); 8]);
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

            if let Some(mut solution) = eight_queens_problem_inner(new_available, idx + 1) {
                unsafe {
                    solution.get_unchecked_mut(idx).write(to_place);
                }
                return Some(solution);
            }
        }

        None
    }
}

pub mod with_boardset_tinyvec {
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

pub mod with_hashset_cached_allocs {
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

        let mut new_available = HashSet::with_capacity(available.len());
        for to_place in available.iter().copied() {
            new_available.clone_from(available);
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

pub mod with_fxhashset {
    use fxhash::FxHashSet;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res =
            eight_queens_problem_inner(&FxHashSet::from_iter(BoardSet::all().iter()), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &FxHashSet<BoardIdx>,
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

pub mod with_specialhashset {
    use std::collections::HashSet;

    use crate::eight_queens::{BoardIdx, BoardSet, SpecialHashBuildHasher};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res =
            eight_queens_problem_inner(&HashSet::from_iter(BoardSet::all().iter()), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &HashSet<BoardIdx, SpecialHashBuildHasher>,
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

pub mod with_btreeset {
    use std::collections::BTreeSet;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res =
            eight_queens_problem_inner(&BTreeSet::from_iter(BoardSet::all().iter()), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &BTreeSet<BoardIdx>,
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

pub mod with_vec {
    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let res = eight_queens_problem_inner(&Vec::from_iter(BoardSet::all().iter()), 8).unwrap();
        res.try_into().unwrap()
    }

    /// Returns `Some(_)` with a list of `to_place` queens, or `None` if it failed
    fn eight_queens_problem_inner(
        available: &Vec<BoardIdx>,
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
                let Some(to_remove) = new_available.iter().position(|x| i == *x) else {
                    continue;
                };
                // `remove` is measured to be faster than `swap_remove`
                // This might have to do with the order in which we tend to access the indices
                new_available.remove(to_remove);
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

pub mod with_iter {
    use std::collections::HashSet;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let mut selected: Vec<BoardIdx> = Vec::with_capacity(8);
        let mut to_attempt: Vec<HashSet<BoardIdx>> =
            vec![HashSet::from_iter(BoardSet::all().iter())];

        loop {
            if selected.len() == 8 {
                return selected.try_into().unwrap();
            }

            let available = to_attempt.last_mut().unwrap();
            let Some(chosen) = available.iter().copied().next() else {
                to_attempt.pop();
                selected.pop();
                continue;
            };
            available.remove(&chosen);
            selected.push(chosen);

            let mut sub_available = available.clone();

            chosen
                .iter_diagonals()
                .into_iter()
                .chain(chosen.iter_col())
                .chain(chosen.iter_row())
                .for_each(|r| {
                    sub_available.remove(&r);
                });

            to_attempt.push(sub_available);
        }
    }
}

pub mod with_iter_boardset {
    use std::collections::HashSet;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let mut selected: Vec<BoardIdx> = Vec::with_capacity(8);
        let mut to_attempt: Vec<BoardSet> = vec![BoardSet::all()];

        loop {
            if selected.len() == 8 {
                return selected.try_into().unwrap();
            }

            let available = to_attempt.last_mut().unwrap();
            let Some(chosen) = available.iter().into_iter().next() else {
                to_attempt.pop();
                selected.pop();
                continue;
            };
            available.remove(chosen);
            selected.push(chosen);

            let mut sub_available = available.clone();

            chosen
                .iter_diagonals()
                .into_iter()
                .chain(chosen.iter_col())
                .chain(chosen.iter_row())
                .for_each(|r| sub_available.remove(r));

            to_attempt.push(sub_available);
        }
    }
}

pub mod with_iter_boardset_cursor {
    use fastrand::Rng;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        let mut rng = Rng::new();
        let mut selected: Vec<BoardIdx> = Vec::with_capacity(8);
        let mut available_stack: Vec<(BoardSet, u8)> = vec![(BoardSet::all(), rng.u8(0..64))];

        'outer: loop {
            if selected.len() == 8 {
                return selected.try_into().unwrap();
            }

            let (available, available_cursor) = available_stack.last_mut().unwrap();

            // Advance cursor until it points to an available board space
            let mut count = 0;
            loop {
                if available.contains_idx(*available_cursor) {
                    break;
                }

                if count >= 64 {
                    available_stack.pop();
                    selected.pop();
                    continue 'outer;
                }

                *available_cursor = (*available_cursor + 1) % 64;

                count += 1;
            }

            let chosen_idx = *available_cursor;
            let chosen = BoardSet::idx_to_board_idx(chosen_idx);
            available.remove(chosen);
            selected.push(chosen);

            let mut sub_available = available.clone();

            chosen
                .iter_diagonals()
                .into_iter()
                .chain(chosen.iter_col())
                .chain(chosen.iter_row())
                .for_each(|r| sub_available.remove(r));

            available_stack.push((sub_available, rng.u8(0..64)));
        }
    }
}

pub mod with_iter_boardset_cursor_laneopts {
    use fastrand::Rng;

    use crate::eight_queens::{BoardIdx, BoardSet};

    pub fn eight_queens_problem() -> [BoardIdx; 8] {
        /// Index with `BoardSet::board_idx_to_idx`
        ///
        /// Gives the BoardSet that the given queen can***not*** see
        const QUEEN_VISION_LOOKUP_TABLE: [BoardSet; 64] = {
            let mut table = [BoardSet::none(); 64];
            let mut idx = 0;
            while idx < 64 {
                let queen = BoardSet::idx_to_board_idx(idx);
                let mut vision = BoardSet::all();
                vision.remove_col(queen.col);
                vision.remove_row(queen.row);

                macro_rules! eliminate_diagonal {
                    ($dx:expr, $dy:expr) => {{
                        let mut seen = queen;
                        while seen.is_valid() {
                            vision.remove(seen);
                            seen.col = seen.col.wrapping_add_signed($dx);
                            seen.row = seen.row.wrapping_add_signed($dy);
                        }
                    }};
                }

                eliminate_diagonal!(1, 1);
                eliminate_diagonal!(1, -1);
                eliminate_diagonal!(-1, 1);
                eliminate_diagonal!(-1, -1);

                table[idx as usize] = vision;

                idx += 1;
            }
            table
        };

        let mut rng = Rng::new();
        let mut selected: Vec<BoardIdx> = Vec::with_capacity(8);
        let mut available_stack: Vec<(BoardSet, u8)> = Vec::with_capacity(8);
        available_stack.push((BoardSet::all(), rng.u8(0..64)));

        loop {
            if selected.len() == 8 {
                return selected.try_into().unwrap();
            }

            let (available, available_cursor) = available_stack.last_mut().unwrap();

            // Advance cursor until it points to an available board space
            // We can use trailing_zeros to find the next available space from the cursor, and if we rotate based
            // on the current cursor we can change the starting location
            // This feels so smart wtf
            {
                let rotated_board = available.inner.rotate_right(*available_cursor as u32);
                let advance_by = rotated_board.trailing_zeros();
                if advance_by == 64 {
                    available_stack.pop();
                    selected.pop();
                    continue;
                }
                *available_cursor = (*available_cursor + advance_by as u8) % 64;
            }

            let chosen_idx = *available_cursor;
            let chosen = BoardSet::idx_to_board_idx(chosen_idx);
            available.remove(chosen);
            selected.push(chosen);

            let mut sub_available = available.clone();

            sub_available = sub_available
                & QUEEN_VISION_LOOKUP_TABLE[BoardSet::board_idx_to_idx(chosen) as usize];

            available_stack.push((sub_available, rng.u8(0..64)));
        }
    }
}

#[cfg(test)]
fn queens_conflict(queens: impl IntoIterator<Item = BoardIdx>) -> bool {
    let queens = queens.into_iter().collect::<Vec<_>>();
    for queen in &queens {
        if queen
            .iter_diagonals()
            .into_iter()
            .chain(queen.iter_col())
            .chain(queen.iter_row())
            .any(|intersects| (intersects != *queen) && queens.contains(&intersects))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_all() {
        use_with_all! {
            println!("Testing {IMP_NAME}...");
            for _ in 0..16 {
                let queens = imp::eight_queens_problem();
                assert!(!crate::eight_queens::queens_conflict(queens));
            }
        };
    }
}
