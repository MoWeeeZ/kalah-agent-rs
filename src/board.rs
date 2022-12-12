use std::fmt::{Debug, Display};

type House = u16;
type HouseNum = u8;

/*====================================================================================================================*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    White,
    Black,
}

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
        if ((self.data & 0b1000_0000) >> 7) == 0 {
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

#[derive(Clone)]
pub struct Board {
    houses: Box<[House]>,

    pub our_store: u16,
    pub their_store: u16,

    flipped: bool,
}

impl Board {
    pub fn new(h: u8, s: House) -> Self {
        assert!(h <= 128, "Can't create more than 128 houses");

        let houses = vec![s; 2 * h as usize].into_boxed_slice();
        Board {
            // houses: vec![s; 2 * h as usize],
            houses,
            our_store: 0,
            their_store: 0,
            flipped: false,
        }
    }

    pub fn h(&self) -> u8 {
        (self.houses.len() / 2) as u8
    }

    pub fn our_houses(&self) -> &[House] {
        let h = self.h() as usize;
        &self.houses[..h]
    }

    pub fn our_houses_mut(&mut self) -> &mut [House] {
        let h = self.h() as usize;
        &mut self.houses[..h]
    }

    pub fn their_houses(&self) -> &[House] {
        let h = self.h() as usize;
        &self.houses[h..]
    }

    pub fn their_houses_mut(&mut self) -> &mut [House] {
        let h = self.h() as usize;
        &mut self.houses[h..]
    }

    pub fn flipped(&self) -> bool {
        self.flipped
    }

    pub fn flip_board(&mut self) {
        let h = self.h() as usize;

        unsafe {
            for i in 0..h {
                std::ptr::swap(self.houses.as_mut_ptr().add(i), self.houses.as_mut_ptr().add(h + i));
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
                if self.our_houses()[i] == 1 {
                    self.our_store += self.our_houses()[i];
                    self.our_houses_mut()[i] = 0;
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
                return true;
            }

            // distribute seeds to their houses
            for j in 0..h {
                self.their_houses_mut()[j] += 1;
                seeds_in_hand -= 1;

                if seeds_in_hand == 0 {
                    return false;
                }
            }

            // don't distribute seeds to their store

            // distribute seeds to our houses
            for i in 0..h {
                self.our_houses_mut()[i] += 1;
                seeds_in_hand -= 1;

                if seeds_in_hand == 0 {
                    if self.our_houses()[i] == 1 {
                        self.our_store += self.our_houses()[i];
                        self.our_houses_mut()[i] = 0;
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

        for house in self.houses.iter_mut() {
            *house = 0;
        }
    }

    pub fn value_heuristic(&self) -> Valuation {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        // const EPS: f32 = 1.0 / u16::MAX as f32;
        const EPS: f32 = 1e-5;

        let our_store = self.our_store as f32;
        let their_store = self.their_store as f32;

        let our_houses_sum = self.our_houses().iter().sum::<u16>() as f32;
        let their_houses_sum = self.their_houses().iter().sum::<u16>() as f32;

        if !self.has_legal_move() {
            // no move left -> this is a terminal node
            // meaning the player with more seeds in their store wins the game
            // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
            // and vice versa. If both have the same number, it's a draw with value 0.0
            let score_diff = our_store + our_houses_sum - their_store - their_houses_sum;

            return match score_diff {
                val if val > 0.0 => TerminalWhiteWin { plies: 0 },
                val if val < 0.0 => TerminalBlackWin { plies: 0 },
                val if val == 0.0 => TerminalDraw { plies: 0 },
                val => panic!("Value has invalid value {}", val),
            };
        }

        let score = ((1.0 + EPS) * our_store /* * our_store */ + our_houses_sum)
            - ((1.0 + EPS) * their_store /* * their_store */ + their_houses_sum);

        NonTerminal { value: score.tanh() }
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

/*====================================================================================================================*/

/// assertion: value will never be f32::NAN, making it comparable
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Valuation {
    NonTerminal { value: f32 },
    TerminalWhiteWin { plies: u32 },
    TerminalBlackWin { plies: u32 },
    TerminalDraw { plies: u32 },
}

impl Valuation {
    pub fn advance_step(self) -> Valuation {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match self {
            NonTerminal { .. } => self,
            TerminalWhiteWin { plies: steps } => TerminalWhiteWin { plies: steps + 1 },
            TerminalBlackWin { plies: steps } => TerminalBlackWin { plies: steps + 1 },
            TerminalDraw { plies: steps } => TerminalDraw { plies: steps + 1 },
        }
    }
}

/// flip the player perspective of the valuation
impl std::ops::Neg for Valuation {
    type Output = Valuation;

    fn neg(self) -> Self::Output {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match self {
            NonTerminal { value } => NonTerminal { value: -value },
            TerminalWhiteWin { plies: steps } => TerminalBlackWin { plies: steps },
            TerminalBlackWin { plies: steps } => TerminalWhiteWin { plies: steps },
            TerminalDraw { plies: steps } => TerminalDraw { plies: steps },
        }
    }
}

impl Eq for Valuation {
    fn assert_receiver_is_total_eq(&self) {}
}

impl PartialOrd for Valuation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// compare valuations from the perspective of White
impl Ord for Valuation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::{Greater, Less};
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match (self, other) {
            // pick Valuation with higher value
            (NonTerminal { value: val1 }, NonTerminal { value: val2 }) => val1.partial_cmp(val2).unwrap(),
            (NonTerminal { .. }, TerminalWhiteWin { .. }) => Less,
            (NonTerminal { .. }, TerminalBlackWin { .. }) => Greater,
            (NonTerminal { value }, TerminalDraw { .. }) => value.partial_cmp(&0.0).unwrap(),
            (TerminalWhiteWin { .. }, NonTerminal { .. }) => Greater,
            // pick Valuation with less steps until win
            (TerminalWhiteWin { plies: s1 }, TerminalWhiteWin { plies: s2 }) => s1.cmp(s2).reverse(),
            (TerminalWhiteWin { .. }, TerminalBlackWin { .. }) => Greater,
            (TerminalWhiteWin { .. }, TerminalDraw { .. }) => Greater,
            (TerminalBlackWin { .. }, NonTerminal { .. }) => Less,
            (TerminalBlackWin { .. }, TerminalWhiteWin { .. }) => Less,
            // pick Valuation with more steps until loss: opponent might make mistake and lose certain win
            (TerminalBlackWin { plies: s1 }, TerminalBlackWin { plies: s2 }) => s1.cmp(s2),
            (TerminalBlackWin { .. }, TerminalDraw { .. }) => Less,
            (TerminalDraw { .. }, NonTerminal { value }) => (0.0).partial_cmp(value).unwrap(),
            (TerminalDraw { .. }, TerminalWhiteWin { .. }) => Less,
            (TerminalDraw { .. }, TerminalBlackWin { .. }) => Greater,
            // pick Valuation with more steps until draw: opponent might make mistake and lose certain draw
            (TerminalDraw { plies: s1 }, TerminalDraw { plies: s2 }) => s1.cmp(s2).reverse(),
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
}
