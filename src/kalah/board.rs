use std::fmt::{Debug, Display};

pub type House = u16;

/*====================================================================================================================*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    White,
    Black,
}

// flip the player, i.e. White -> Black and Black -> White
impl std::ops::Not for Player {
    type Output = Player;

    fn not(self) -> Self::Output {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::White => write!(f, "White"),
            Player::Black => write!(f, "Black"),
        }
    }
}

/*====================================================================================================================*/

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
    // bytes 0..6 : number of house the move starts from
    // bytes 7 : whether the move is by White or Black
    data: u8,
}

impl Move {
    pub fn new(house_num: u8, player: Player) -> Self {
        assert!(house_num < 128, "House needs to be smaller than 128");

        let mut data = house_num;
        match player {
            Player::White => {}
            Player::Black => data |= 1 << 7,
        };
        Move { data }
    }

    pub fn house(&self) -> u8 {
        self.data & 0b0111_1111
    }

    pub fn player(&self) -> Player {
        if (self.data & 0b1000_0000) == 0 {
            Player::White
        } else {
            Player::Black
        }
    }

    pub fn flip_player(&self) -> Move {
        Move::new(self.house(), !self.player())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.house() + 1)
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Move({}, {})", self.house(), self.player())
    }
}

/*====================================================================================================================*/

// should be 24 bytes in size
pub struct Board {
    h: u8,

    our_houses_ptr: *mut House,
    their_houses_ptr: *mut House,

    pub our_store: u16,
    pub their_store: u16,

    flipped: bool,
}

unsafe impl Send for Board {}
unsafe impl Sync for Board {}

impl Board {
    pub fn from_parts(
        h: u8,
        our_houses: Vec<House>,
        their_houses: Vec<House>,
        our_store: House,
        their_store: House,
        flipped: bool,
    ) -> Self {
        assert!(h <= 128, "Can't create more than 128 houses");

        assert_eq!(our_houses.len(), h as usize);
        assert_eq!(their_houses.len(), h as usize);

        let mut houses_vec: Vec<u16> = Vec::with_capacity(2 * h as usize);
        assert_eq!(houses_vec.capacity(), 2 * h as usize);

        houses_vec.extend_from_slice(&our_houses);
        houses_vec.extend_from_slice(&their_houses);

        assert_eq!(houses_vec.len(), 2 * h as usize);

        let houses_ptr = houses_vec.as_mut_ptr();
        std::mem::forget(houses_vec);

        let our_houses_ptr = houses_ptr;
        let their_houses_ptr = unsafe { houses_ptr.add(h as usize) };

        Board {
            h,
            our_houses_ptr,
            their_houses_ptr,
            our_store,
            their_store,
            flipped,
        }
    }

    pub fn new(h: u8, s: House) -> Self {
        Board::from_parts(h, vec![s; h as usize], vec![s; h as usize], 0, 0, false)
    }

    pub fn from_kpg(kpg: &str) -> Self {
        let kpg: String = kpg.chars().filter(|c| !c.is_whitespace()).collect();

        let mut nums = kpg.strip_prefix('<').unwrap().strip_suffix('>').unwrap().split(',');

        let h: u8 = nums.next().unwrap().parse().unwrap();

        let our_store: u16 = nums.next().unwrap().parse().unwrap();
        let their_store: u16 = nums.next().unwrap().parse().unwrap();

        // let houses_vec: Vec<u16> = nums.map(|num_s| num_s.parse().unwrap()).collect();
        let mut our_houses_vec: Vec<House> = Vec::with_capacity(h as usize);
        for _ in 0..h {
            our_houses_vec.push(nums.next().unwrap().parse().unwrap());
        }

        let mut their_houses_vec: Vec<House> = Vec::with_capacity(h as usize);
        for _ in 0..h {
            their_houses_vec.push(nums.next().unwrap().parse().unwrap());
        }

        assert_eq!(nums.count(), 0);

        Board::from_parts(h, our_houses_vec, their_houses_vec, our_store, their_store, false)
    }

    /// clone other into self, overwriting the old values, but not reallocating memory
    pub fn clone_from(&mut self, other: &Board) {
        assert!(self.h == other.h, "Tried to clone_from board of different h");

        let h = self.h as usize;

        unsafe {
            std::ptr::copy_nonoverlapping(other.our_houses_ptr, self.our_houses_ptr, h);
            std::ptr::copy_nonoverlapping(other.their_houses_ptr, self.their_houses_ptr, h);
        }

        self.our_store = other.our_store;
        self.their_store = other.their_store;

        self.flipped = other.flipped
    }

    pub fn to_kgp(&self) -> String {
        use std::fmt::Write;

        let mut s = String::new();

        match self.flipped {
            false => {
                write!(s, "<{}, {}, {}", self.h(), self.our_store, self.their_store).unwrap();

                for seed in self.our_houses() {
                    write!(s, ", {}", seed).unwrap();
                }
                for seed in self.their_houses() {
                    write!(s, ", {}", seed).unwrap();
                }

                write!(s, ">").unwrap();
            }
            true => {
                write!(s, "<{}, {}, {}", self.h(), self.their_store, self.our_store).unwrap();

                for seed in self.their_houses() {
                    write!(s, ", {}", seed).unwrap();
                }
                for seed in self.our_houses() {
                    write!(s, ", {}", seed).unwrap();
                }

                write!(s, ">").unwrap();
            }
        }

        s
    }

    pub fn h(&self) -> u8 {
        self.h
    }

    pub fn our_store(&self) -> u16 {
        self.our_store
    }

    pub fn their_store(&self) -> u16 {
        self.their_store
    }

    pub fn our_houses(&self) -> &[House] {
        // let h = self.h() as usize;
        // &self.houses[..h]
        unsafe { std::slice::from_raw_parts(self.our_houses_ptr, self.h as usize) }
    }

    pub fn our_houses_mut(&mut self) -> &mut [House] {
        // let h = self.h() as usize;
        // &mut self.houses[..h]
        unsafe { std::slice::from_raw_parts_mut(self.our_houses_ptr, self.h as usize) }
    }

    pub fn their_houses(&self) -> &[House] {
        // let h = self.h() as usize;
        // &self.houses[h..]
        unsafe { std::slice::from_raw_parts(self.their_houses_ptr, self.h as usize) }
    }

    pub fn their_houses_mut(&mut self) -> &mut [House] {
        // let h = self.h() as usize;
        // &mut self.houses[h..]
        unsafe { std::slice::from_raw_parts_mut(self.their_houses_ptr, self.h as usize) }
    }

    pub fn flipped(&self) -> bool {
        self.flipped
    }

    pub fn flip_board(&mut self) {
        std::mem::swap(&mut self.our_houses_ptr, &mut self.their_houses_ptr);

        std::mem::swap(&mut self.our_store, &mut self.their_store);

        self.flipped = !self.flipped
    }

    pub fn apply_move(&mut self, move_: Move) -> bool {
        assert!(
            move_.house() < self.h(),
            "Trying to apply move {} that is out of range",
            move_
        );

        if move_.player() == Player::Black {
            // if the move is by 'Black': flip the board, apply the move as if by White, flip the board again
            self.flip_board();
            let ret = self.apply_move(move_.flip_player());
            self.flip_board();
            return ret;
        }

        let h = self.h() as u16;

        let start_house = move_.house() as usize;

        let seeds_in_hand = self.our_houses()[start_house];
        self.our_houses_mut()[start_house] = 0;

        assert!(seeds_in_hand != 0, "Trying to move out of empty house");

        // number of all houses we distribute seeds to:
        // h x our houses, 1 x our store, h x their houses
        let cycle_length = 2 * h + 1;

        // number of complete cycles we make: can add this value to all houses and our store
        let num_cycles = seeds_in_hand / cycle_length;
        // number of seeds remaining after complete cycles have been made
        let mut rem = (seeds_in_hand % cycle_length) as usize;

        if seeds_in_hand > cycle_length {
            // distribute seeds to all houses and our store evenly
            for our_house in self.our_houses_mut() {
                *our_house += num_cycles;
            }

            self.our_store += num_cycles;

            for their_house in self.their_houses_mut() {
                *their_house += num_cycles;
            }
        }

        // our houses after starting house
        for our_house in self
            .our_houses_mut()
            .iter_mut()
            .skip(start_house + 1) // skip until after starting house
            .take(rem)
        {
            *our_house += 1;
            rem -= 1;
        }

        // our store
        if rem > 0 {
            self.our_store += 1;
            rem -= 1;
        }

        // their houses
        for their_house in self.their_houses_mut().iter_mut().take(rem) {
            *their_house += 1;
            rem -= 1;
        }

        // our houses until starting house (inclusive)
        if rem > 0 {
            for our_house in self.our_houses_mut().iter_mut().take(rem) {
                *our_house += 1;
                rem -= 1;
            }
        }

        assert_eq!(rem, 0);

        // index of last house:
        // 0..h : our_houses[i]
        // h : our_store
        // (h+1)..(2h+1) : their_house[i - h - 1] (not relevant)
        let h = h as usize; // only used for indexing from here on, so 'convert' to usize once
        let last_house_idx = (start_house + seeds_in_hand as usize) % cycle_length as usize;

        // last seed in our house && our house was empty && opposite house if not empty:
        if last_house_idx < h
            && self.our_houses()[last_house_idx] == 1
            && self.their_houses()[h - last_house_idx - 1] > 0
        {
            self.our_store += self.their_houses()[h - last_house_idx - 1] + 1;
            self.our_houses_mut()[last_house_idx] = 0;
            self.their_houses_mut()[h - last_house_idx - 1] = 0;
        }

        if !self.has_legal_move() {
            // if no moves remain: finish the board
            self.finish_game();
        }

        // if last seed in our store -> true (bonus move), else -> false
        last_house_idx == h
    }

    pub fn legal_moves(&self, player: Player) -> Vec<Move> {
        let houses = match player {
            Player::White => self.our_houses(),
            Player::Black => self.their_houses(),
        };

        houses
            .iter()
            .enumerate()
            .filter(|&(_house_num, &house)| house != 0)
            .map(|(house_num, _house)| Move::new(house_num as u8, player))
            .collect()
    }

    pub fn is_legal_move(&self, move_: Move) -> bool {
        match move_.player() {
            Player::White => self.our_houses()[move_.house() as usize] != 0,
            Player::Black => self.their_houses()[move_.house() as usize] != 0,
        }
    }

    pub fn has_legal_move(&self) -> bool {
        self.our_houses().iter().any(|&house| house != 0) && self.their_houses().iter().any(|&house| house != 0)
    }

    pub fn finish_game(&mut self) {
        self.our_store += self.our_houses().iter().sum::<u16>();
        self.their_store += self.their_houses().iter().sum::<u16>();

        self.our_houses_mut().fill(0);
        self.their_houses_mut().fill(0);
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>3} |", self.their_store)?;

        for their_house in self.their_houses().iter().rev() {
            write!(f, " {:>3}", *their_house)?;
        }

        write!(f, "\n\n      ")?;

        for our_house in self.our_houses() {
            write!(f, "{:>3} ", our_house)?;
        }

        write!(f, "| {:>3}", self.our_store)
    }
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_kgp())
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        /* // recreate houses Vec
        let houses = unsafe { Vec::from_raw_parts(self.houses_ptr, 2 * self.h as usize, 2 * self.h as usize) };

        // clone houses Vec and get pointer to its buffer
        let mut houses_clone = houses.clone();
        assert!(houses_clone.capacity() == 2 * self.h as usize);
        let houses_clone_ptr = houses_clone.as_mut_ptr();

        // forget houses and houses_clone Vecs
        std::mem::forget(houses);
        std::mem::forget(houses_clone); */

        let h = self.h as usize;

        let mut houses_vec: Vec<House> = Vec::with_capacity(2 * h);
        let our_houses_ptr = houses_vec.as_mut_ptr();
        let their_houses_ptr = unsafe { our_houses_ptr.add(h) };
        std::mem::forget(houses_vec);

        unsafe {
            std::ptr::copy_nonoverlapping(self.our_houses_ptr, our_houses_ptr, h);
            std::ptr::copy_nonoverlapping(self.their_houses_ptr, their_houses_ptr, h);
        }

        Self {
            our_houses_ptr,
            their_houses_ptr,
            our_store: self.our_store,
            their_store: self.their_store,
            h: self.h,
            flipped: self.flipped,
        }
    }
}

impl Drop for Board {
    fn drop(&mut self) {
        // recreate houses Vec and drop it
        unsafe {
            // beginning of the buffer is the lower of the two addresses
            let houses_ptr = if self.our_houses_ptr < self.their_houses_ptr {
                self.our_houses_ptr
            } else {
                self.their_houses_ptr
            };
            let houses_vec = Vec::from_raw_parts(houses_ptr, 2 * self.h as usize, 2 * self.h as usize);
            drop(houses_vec);
        }
    }
}

/*====================================================================================================================*/

#[cfg(test)]
mod tests {
    use crate::Board;

    #[test]
    fn test_board_new() {
        let h = 6;
        let s = 4;

        let board = Board::new(h, s);

        assert!(board.h() == h);

        for our_house in board.our_houses() {
            assert!(*our_house == s);
        }

        for their_house in board.their_houses() {
            assert!(*their_house == s);
        }
    }

    #[test]
    fn test_board_flip() {
        let mut board = Board::new(6, 4);

        for (i, our_house) in board.our_houses_mut().iter_mut().enumerate() {
            *our_house = i as u16;
        }

        for (i, their_house) in board.their_houses_mut().iter_mut().enumerate() {
            *their_house = i as u16 + 10;
        }

        board.our_store = 42;
        board.their_store = 24;

        board.flip_board();

        for (i, our_house) in board.our_houses().iter().enumerate() {
            assert!(*our_house == i as u16 + 10);
        }

        for (i, their_house) in board.their_houses().iter().enumerate() {
            assert!(*their_house == i as u16);
        }

        assert!(board.our_store == 24);
        assert!(board.their_store == 42);
    }

    #[test]
    fn test_from_to_kpg() {
        let kpg = "<3, 2, 3, 11, 12, 13, 21, 22, 23>";

        let board = Board::from_kpg(kpg);

        assert_eq!(board.h(), 3);

        assert_eq!(board.our_store(), 2);
        assert_eq!(board.their_store(), 3);

        assert_eq!(board.our_houses(), &[11, 12, 13]);
        assert_eq!(board.their_houses(), &[21, 22, 23]);

        assert_eq!(board.to_kgp(), kpg);
    }
}
