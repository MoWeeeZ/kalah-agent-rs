use std::fmt::Display;

use crate::{Board, House};

/// # Safety
///
/// - value shall never be f32::NAN, to making it comparable using f32::partial_cmp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valuation {
    NonTerminal { value: i32 },
    TerminalWhiteWin { plies: u32 },
    TerminalBlackWin { plies: u32 },
    TerminalDraw { plies: u32 },
}

impl Valuation {
    /* pub fn is_terminal(&self) -> bool {
        !(matches!(self, Valuation::NonTerminal { .. }))
    } */

    pub fn increase_plies(self) -> Valuation {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match self {
            NonTerminal { .. } => self,
            TerminalWhiteWin { plies: steps } => TerminalWhiteWin { plies: steps + 1 },
            TerminalBlackWin { plies: steps } => TerminalBlackWin { plies: steps + 1 },
            TerminalDraw { plies: steps } => TerminalDraw { plies: steps + 1 },
        }
    }
}

impl Display for Valuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Valuation::NonTerminal { value } => write!(f, "{}", value),
            Valuation::TerminalWhiteWin { plies } => write!(f, "WhiteWin({})", plies),
            Valuation::TerminalBlackWin { plies } => write!(f, "BlackWin({})", plies),
            Valuation::TerminalDraw { plies } => write!(f, "Draw({})", plies),
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

impl PartialOrd for Valuation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// compare valuations from the perspective of White
impl Ord for Valuation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::{Greater, Less};
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match (self, other) {
            // TerminalWhiteWin can only be beaten by TerminalWhiteWin with less plies
            (TerminalWhiteWin { plies: p1 }, TerminalWhiteWin { plies: p2 }) => p1.cmp(p2).reverse(),
            (TerminalWhiteWin { .. }, _) => Greater,
            (_, TerminalWhiteWin { .. }) => Less,

            // NonTerminal and TerminalDraw get compared by value (draws count as 0 value)
            (NonTerminal { value: v1 }, NonTerminal { value: v2 }) => v1.cmp(v2),
            (NonTerminal { value }, TerminalDraw { .. }) => value.cmp(&0),
            (TerminalDraw { .. }, NonTerminal { value }) => 0.cmp(value),
            // select longer draw: more chances for opponent to mess up
            (TerminalDraw { plies: p1 }, TerminalDraw { plies: p2 }) => p1.cmp(p2),

            // TerminalBlackWin can only beat a TerminalBlackWin with less plies
            (TerminalBlackWin { plies: p1 }, TerminalBlackWin { plies: p2 }) => p1.cmp(p2),
            (TerminalBlackWin { .. }, _) => Less,
            (_, TerminalBlackWin { .. }) => Greater,
        }
    }
}

/*====================================================================================================================*/

pub type ValuationFn = fn(&Board) -> Valuation;

#[allow(dead_code)]
pub fn store_diff_valuation(board: &Board) -> Valuation {
    use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

    let our_store = board.our_store as i32;
    let their_store = board.their_store as i32;

    let store_diff = our_store - their_store;

    if !board.has_legal_move() {
        // no move left or more than half the seeds in one players store -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
        // and vice versa. If both have the same number, it's a draw with value 0.0

        return match store_diff {
            store_diff if store_diff > 0 => TerminalWhiteWin { plies: 0 },
            store_diff if store_diff < 0 => TerminalBlackWin { plies: 0 },
            store_diff if store_diff == 0 => TerminalDraw { plies: 0 },
            _ => unreachable!(),
        };
    }

    NonTerminal { value: store_diff }
}

#[allow(dead_code)]
pub fn store_diff_valuation2(board: &Board) -> Valuation {
    use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

    let our_store = board.our_store as i32;
    let their_store = board.their_store as i32;

    let our_houses_sum = board.our_houses().iter().sum::<u16>() as i32;
    let their_houses_sum = board.their_houses().iter().sum::<u16>() as i32;

    let half_total_seeds = (our_store + our_houses_sum + their_store + their_houses_sum) / 2;

    let store_diff = our_store - their_store;

    if !board.has_legal_move() || our_store > half_total_seeds || their_store > half_total_seeds {
        // no move left or more than half the seeds in one players store -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
        // and vice versa. If both have the same number, it's a draw with value 0.0

        return match store_diff {
            store_diff if store_diff > 0 => TerminalWhiteWin { plies: 0 },
            store_diff if store_diff < 0 => TerminalBlackWin { plies: 0 },
            store_diff if store_diff == 0 => TerminalDraw { plies: 0 },
            _ => unreachable!(),
        };
    }

    NonTerminal { value: store_diff }
}

#[allow(dead_code)]
pub fn seed_diff_valuation(board: &Board) -> Valuation {
    use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

    let our_store = board.our_store as i32;
    let their_store = board.their_store as i32;

    let our_houses_sum = board.our_houses().iter().sum::<House>() as i32;
    let their_houses_sum = board.their_houses().iter().sum::<House>() as i32;

    if !board.has_legal_move() {
        // no move left or more than half the seeds in one players store -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
        // and vice versa. If both have the same number, it's a draw with value 0.0
        let store_diff = our_store - their_store;

        return match store_diff {
            store_diff if store_diff > 0 => TerminalWhiteWin { plies: 0 },
            store_diff if store_diff < 0 => TerminalBlackWin { plies: 0 },
            store_diff if store_diff == 0 => TerminalDraw { plies: 0 },
            // val => panic!("Value has invalid value {}", val),
            _ => unreachable!(),
        };
    }

    // let total_seeds = our_store + our_houses_sum + their_store + their_houses_sum;

    // let score = ((1.0 + EPS) * our_store + our_houses_sum) - ((1.0 + EPS) * their_store + their_houses_sum);

    let seed_diff = our_store + our_houses_sum - their_store - their_houses_sum;
    // let store_diff = our_store - their_store;

    // upper 16 bits: seed difference
    // lower 16 bits: store difference (as tie breaker), shifted to the positive range
    // let score = ((seed_diff as i32) << 16) + (store_diff as i32 - (i16::MIN as i32));
    let score = seed_diff;

    NonTerminal { value: score }
}

/*====================================================================================================================*/

#[cfg(test)]
mod tests {
    use super::Valuation;

    #[test]
    fn test_cmp() {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        let nt1 = NonTerminal { value: -5 };
        let nt2 = NonTerminal { value: 5 };

        let ww1 = TerminalWhiteWin { plies: 5 };
        let ww2 = TerminalWhiteWin { plies: 10 };

        let bw1 = TerminalBlackWin { plies: 5 };
        let bw2 = TerminalBlackWin { plies: 10 };

        let draw1 = TerminalDraw { plies: 5 };
        let draw2 = TerminalDraw { plies: 10 };

        assert!(nt1 < nt2);
        assert!(ww1 > ww2);
        assert!(bw1 < bw2);
        assert!(draw1 < draw2);

        assert!(bw1 < nt1);
        assert!(nt1 < draw1);
        assert!(draw1 < nt2);
        assert!(nt2 < ww1);

        assert!(bw1 < draw1);
        assert!(draw1 < ww1);
        assert!(bw1 < ww1);
    }
}
