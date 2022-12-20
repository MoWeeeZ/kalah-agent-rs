use crate::Board;

/* pub trait Valuator {
    fn valuation(board: &Board) -> Valuation;
}

pub struct StoreDiffValuator {} */

pub type ValuationFn = fn(&Board) -> Valuation;

#[allow(dead_code)]
pub fn store_diff_valuation(board: &Board) -> Valuation {
    use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

    // const EPS: f32 = 1.0 / u16::MAX as f32;
    const EPS: f32 = 1e-5;

    let our_store = board.our_store as f32;
    let their_store = board.their_store as f32;

    let our_houses_sum = board.our_houses().iter().sum::<u16>() as f32;
    let their_houses_sum = board.their_houses().iter().sum::<u16>() as f32;

    if !board.has_legal_move() {
        // no move left or more than half the seeds in one players store -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
        // and vice versa. If both have the same number, it's a draw with value 0.0
        let score_diff = our_store - their_store;

        return match score_diff {
            val if val > 0.0 => TerminalWhiteWin { plies: 0 },
            val if val < 0.0 => TerminalBlackWin { plies: 0 },
            val if val == 0.0 => TerminalDraw { plies: 0 },
            val => panic!("Value has invalid value {}", val),
        };
    }

    let total_seeds = our_store + our_houses_sum + their_store + their_houses_sum;

    let score = ((1.0 + EPS) * our_store) - ((1.0 + EPS) * their_store);

    NonTerminal {
        value: score / total_seeds,
        // value: score.tanh(),
    }
}

#[allow(dead_code)]
pub fn seed_diff_valuation(board: &Board) -> Valuation {
    use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

    // const EPS: f32 = 1.0 / u16::MAX as f32;
    const EPS: f32 = 1e-5;

    let our_store = board.our_store as f32;
    let their_store = board.their_store as f32;

    let our_houses_sum = board.our_houses().iter().sum::<u16>() as f32;
    let their_houses_sum = board.their_houses().iter().sum::<u16>() as f32;

    if !board.has_legal_move() {
        // no move left or more than half the seeds in one players store -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if White has more seeds in the store (i.e. score_diff > 0) this node is a guaranteed win
        // and vice versa. If both have the same number, it's a draw with value 0.0
        let score_diff = our_store - their_store;

        return match score_diff {
            val if val > 0.0 => TerminalWhiteWin { plies: 0 },
            val if val < 0.0 => TerminalBlackWin { plies: 0 },
            val if val == 0.0 => TerminalDraw { plies: 0 },
            val => panic!("Value has invalid value {}", val),
        };
    }

    let total_seeds = our_store + our_houses_sum + their_store + their_houses_sum;

    let score = ((1.0 + EPS) * our_store + our_houses_sum) - ((1.0 + EPS) * their_store + their_houses_sum);

    NonTerminal {
        value: score / total_seeds,
        // value: score.tanh(),
    }
}

/*====================================================================================================================*/

/// # Safety
///
/// - value shall never be f32::NAN, to making it comparable using f32::partial_cmp
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Valuation {
    NonTerminal { value: f32 },
    TerminalWhiteWin { plies: u32 },
    TerminalBlackWin { plies: u32 },
    TerminalDraw { plies: u32 },
}

impl Valuation {
    /* pub fn is_terminal(&self) -> bool {
        !(matches!(self, Valuation::NonTerminal { .. }))
    } */

    pub fn increase_depth(self) -> Valuation {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match self {
            NonTerminal { .. } => self,
            TerminalWhiteWin { plies: steps } => TerminalWhiteWin { plies: steps + 1 },
            TerminalBlackWin { plies: steps } => TerminalBlackWin { plies: steps + 1 },
            TerminalDraw { plies: steps } => TerminalDraw { plies: steps + 1 },
        }
    }

    // Valuation as f32: non-terminals give their inner value, WhiteWin becomes inf, BlackWin -inf, Draw 0
    /* pub fn as_f32(&self) -> f32 {
        use Valuation::{NonTerminal, TerminalBlackWin, TerminalDraw, TerminalWhiteWin};

        match self {
            NonTerminal { value } => *value,
            TerminalWhiteWin { .. } => f32::INFINITY,
            TerminalBlackWin { .. } => f32::NEG_INFINITY,
            TerminalDraw { .. } => 0.0,
        }
    } */
}

impl Eq for Valuation {}

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

// divide non-terminal value by divisor, leave ply count as it is; useful for averaging
impl std::ops::Div<f32> for Valuation {
    type Output = Valuation;

    fn div(self, rhs: f32) -> Self::Output {
        use Valuation::NonTerminal;
        match self {
            NonTerminal { value } => NonTerminal { value: value / rhs },
            _ => self,
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
