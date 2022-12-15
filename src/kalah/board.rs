use std::fmt::{Debug, Display};

type House = u16;
type HouseNum = u8;

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
    pub fn new(house_num: HouseNum, player: Player) -> Self {
        assert!(house_num < 128, "House needs to be smaller than 128");

        let mut data = house_num;
        match player {
            Player::White => {}
            Player::Black => data |= 1 << 7,
        };
        Move { data }
    }

    pub fn house(&self) -> HouseNum {
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

// should be 16 bytes in size
// as we need 9 bytes anyways (for houses and flipped) and the alignment is 8, we can use the remaining 7 padding bytes
pub struct Board {
    // houses: Box<[House]>,
    // reduces the size of Board from 24 bytes to 16 bytes
    // we already know the size of the array is 2*h and we can squeeze it in later as an u8
    houses_ptr: *mut House,

    pub our_store: u16,
    pub their_store: u16,

    h: u8,

    flipped: bool,
}

unsafe impl Send for Board {}
unsafe impl Sync for Board {}

impl Board {
    pub fn new(h: u8, s: House) -> Self {
        assert!(h <= 128, "Can't create more than 128 houses");

        let mut houses_vec: Vec<u16> = vec![s; 2 * h as usize];

        assert!(houses_vec.len() == houses_vec.capacity());

        let houses = houses_vec.as_mut_ptr();

        std::mem::forget(houses_vec);

        Board {
            // houses: vec![s; 2 * h as usize],
            houses_ptr: houses,
            our_store: 0,
            their_store: 0,
            h,
            flipped: false,
        }
    }

    pub fn from_kpg(kpg: &str) -> Self {
        let kpg: String = kpg.chars().filter(|c| !c.is_whitespace()).collect();

        let mut nums = kpg.strip_prefix('<').unwrap().strip_suffix('>').unwrap().split(',');

        let h: u8 = nums.next().unwrap().parse().unwrap();

        let our_store: u16 = nums.next().unwrap().parse().unwrap();
        let their_store: u16 = nums.next().unwrap().parse().unwrap();

        let mut houses_vec: Vec<u16> = nums.map(|num_s| num_s.parse().unwrap()).collect();

        assert_eq!(houses_vec.len(), 2 * h as usize, "{:?}", houses_vec);

        houses_vec.shrink_to_fit();

        assert_eq!(houses_vec.capacity(), 2 * h as usize);

        let houses_ptr = houses_vec.as_mut_ptr();

        std::mem::forget(houses_vec);

        Board {
            houses_ptr,
            our_store,
            their_store,
            h,
            flipped: false,
        }
    }

    pub fn to_kgp(&self) -> String {
        use std::fmt::Write;

        let mut s = String::new();

        match self.flipped {
            false => {
                write!(s, "<{}, {}, {}", self.h(), self.our_store, self.their_store).unwrap();

                for seeds in self.our_houses() {
                    write!(s, ", {}", seeds).unwrap();
                }
                for seeds in self.their_houses() {
                    write!(s, ", {}", seeds).unwrap();
                }

                write!(s, ">").unwrap();
            }
            true => {
                write!(s, "<{}, {}, {}", self.h(), self.their_store, self.our_store).unwrap();

                for seeds in self.their_houses() {
                    write!(s, ", {}", seeds).unwrap();
                }
                for seeds in self.our_houses() {
                    write!(s, ", {}", seeds).unwrap();
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
        unsafe { std::slice::from_raw_parts(self.houses_ptr, self.h as usize) }
    }

    pub fn our_houses_mut(&mut self) -> &mut [House] {
        // let h = self.h() as usize;
        // &mut self.houses[..h]
        unsafe { std::slice::from_raw_parts_mut(self.houses_ptr, self.h as usize) }
    }

    pub fn their_houses(&self) -> &[House] {
        // let h = self.h() as usize;
        // &self.houses[h..]
        unsafe { std::slice::from_raw_parts(self.houses_ptr.add(self.h as usize), self.h as usize) }
    }

    pub fn their_houses_mut(&mut self) -> &mut [House] {
        // let h = self.h() as usize;
        // &mut self.houses[h..]
        unsafe { std::slice::from_raw_parts_mut(self.houses_ptr.add(self.h as usize), self.h as usize) }
    }

    pub fn flipped(&self) -> bool {
        self.flipped
    }

    pub fn flip_board(&mut self) {
        let h = self.h() as usize;

        unsafe {
            for i in 0..h {
                std::ptr::swap(self.houses_ptr.add(i), self.houses_ptr.add(h + i));
            }
        }

        std::mem::swap(&mut self.our_store, &mut self.their_store);

        self.flipped = !self.flipped
    }

    pub fn apply_move(&mut self, move_: Move) -> bool {
        assert!(move_.house() < self.h(), "Trying to apply a move that is out of range");

        if move_.player() == Player::Black {
            // if the move is by 'Black': flip the board, apply the move as if by White, flip the board again
            self.flip_board();
            let ret = self.apply_move(move_.flip_player());
            self.flip_board();
            return ret;
        }

        let h = self.h() as usize;

        let start_house = move_.house() as usize;

        let mut seeds_in_hand = self.our_houses()[start_house];
        self.our_houses_mut()[start_house] = 0;

        assert!(seeds_in_hand != 0, "Trying to move out of empty house");

        for i in (start_house + 1)..h {
            self.our_houses_mut()[i] += 1;
            seeds_in_hand -= 1;

            if seeds_in_hand == 0 {
                // if our house was empty and their opposing house is not
                if self.our_houses()[i] == 1 && self.their_houses_mut()[h - i] != 0 {
                    // move seed from our house and all seeds from their house to our store
                    self.our_store += 1 + self.their_houses()[h - i - 1];

                    self.our_houses_mut()[i] = 0;
                    self.their_houses_mut()[h - i - 1] = 0;
                }

                if !self.has_legal_move() {
                    self.finish_game();
                }

                return false;
            }
        }

        loop {
            // distribute seed to our store
            // seeds_in_hand will never be zero since it's checked for before
            self.our_store += 1;
            seeds_in_hand -= 1;

            if seeds_in_hand == 0 {
                if !self.has_legal_move() {
                    self.finish_game();
                }

                return true;
            }

            // distribute seeds to their houses
            for j in 0..h {
                self.their_houses_mut()[j] += 1;
                seeds_in_hand -= 1;

                if seeds_in_hand == 0 {
                    if !self.has_legal_move() {
                        self.finish_game();
                    }

                    return false;
                }
            }

            // don't distribute seeds to their store

            // distribute seeds to our houses
            for i in 0..h {
                self.our_houses_mut()[i] += 1;
                seeds_in_hand -= 1;

                if seeds_in_hand == 0 {
                    // if our house was empty and their opposing house is not
                    if self.our_houses()[i] == 1 && self.their_houses_mut()[h - i - 1] != 0 {
                        // move seed from our house and all seeds from their house to our store
                        self.our_store += 1 + self.their_houses()[h - i - 1];

                        self.our_houses_mut()[i] = 0;
                        self.their_houses_mut()[h - i - 1] = 0;
                    }

                    if !self.has_legal_move() {
                        self.finish_game();
                    }

                    return false;
                }
            }
        }
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

impl Clone for Board {
    fn clone(&self) -> Self {
        // recreate houses Vec
        let houses = unsafe { Vec::from_raw_parts(self.houses_ptr, 2 * self.h as usize, 2 * self.h as usize) };

        // clone houses Vec and get pointer to its buffer
        let mut houses_clone = houses.clone();
        assert!(houses_clone.capacity() == 2 * self.h as usize);
        let houses_clone_ptr = houses_clone.as_mut_ptr();

        // forget houses and houses_clone Vecs
        std::mem::forget(houses);
        std::mem::forget(houses_clone);

        Self {
            houses_ptr: houses_clone_ptr,
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
        let houses_vec = unsafe { Vec::from_raw_parts(self.houses_ptr, 2 * self.h as usize, 2 * self.h as usize) };
        drop(houses_vec);
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
