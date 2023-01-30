use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::kalah::valuation::{Valuation, ValuationFn};
use crate::{Board, Move, Player, LOG_STATS};

/*====================================================================================================================*/

pub type SharedMinimaxSearchState = Arc<Mutex<MinimaxSearchState>>;

pub struct MinimaxSearchState {
    pub search_active: bool,

    pub principal_variation: Line,
}

pub fn new_shared_minimax_search_state(search_active: bool, principal_variation: Line) -> SharedMinimaxSearchState {
    Arc::new(Mutex::new(MinimaxSearchState {
        search_active,
        principal_variation,
    }))
}

/*====================================================================================================================*/

const LINE_MAX_SIZE: usize = 100;
// type Line = ;

// represents a line of moves
// fixed-sized array so it can be on the stack
#[derive(Clone, Copy)]
pub struct Line {
    len: u32,
    moves: [Move; LINE_MAX_SIZE],
}

impl Line {
    pub fn new() -> Self {
        Line {
            len: 0,
            moves: [Move::new(127, Player::Black); LINE_MAX_SIZE],
        }
    }

    pub fn reset(&mut self) {
        self.len = 0
    }

    pub fn overwrite(&mut self, head: Move, tail: &Line) {
        assert!(tail.len < LINE_MAX_SIZE as u32);

        self.moves[0] = head;
        self.len = 1;

        self.append(tail);
    }

    pub fn append(&mut self, other: &Line) {
        assert!(self.len + other.len < LINE_MAX_SIZE as u32);

        unsafe {
            let src = &other.moves as *const Move;
            let dst = (&mut self.moves as *mut Move).add(self.len as usize);

            std::ptr::copy_nonoverlapping(src, dst, other.len as usize);
        }

        self.len += other.len
    }

    pub fn best_move(&self) -> Option<Move> {
        if self.len > 0 {
            Some(self.moves[0])
        } else {
            None
        }
    }

    pub fn iter(&self) -> std::slice::Iter<Move> {
        self.moves[0..(self.len as usize)].iter()
    }
}

/*====================================================================================================================*/

struct PVSWorker {
    search_state: Arc<Mutex<MinimaxSearchState>>,

    valuation_fn: ValuationFn,

    total_nodes_visited: u64,

    start_t: Instant,
}

impl PVSWorker {
    pub fn new(valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) -> Self {
        PVSWorker {
            search_state,
            valuation_fn,
            total_nodes_visited: 0,
            start_t: Instant::now(),
        }
    }

    fn current_nps(&self) -> f64 {
        self.total_nodes_visited as f64 / self.start_t.elapsed().as_secs_f64()
    }

    fn extend_pv(&mut self, board: &Board, pv: &mut Line) -> Valuation {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut board = board.clone();

        for &move_ in pv.iter() {
            if !board.apply_move(move_) {
                board.flip_board();
            }
        }

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        let mut extend_line = Line::new();

        let value = self.minimax(&board, 1, alpha, beta, &mut extend_line);

        pv.append(&extend_line);

        value
    }

    // stack-based PVS, adapted from https://web.archive.org/web/20040427013839/brucemo.com/compchess/programming/pv.htm
    fn minimax(
        &mut self,
        board: &Board,
        remaining_depth: u32,
        alpha: Valuation,
        beta: Valuation,
        principal_line: &mut Line,
    ) -> Valuation {
        if !self.search_state.lock().unwrap().search_active {
            // search has been ended, search results don't matter anymore, exit thread asap
            return Valuation::NonTerminal { value: 0 };
        }

        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            principal_line.reset();
            return (self.valuation_fn)(board);
        }

        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        let mut board_after_move = board.clone();

        let mut search_line = Line::new();

        for house in 0..board.h() {
            let move_ = Move::new(house, Player::White);

            if !board.is_legal_move(move_) {
                continue;
            }

            // let mut board_after_move = board.clone();
            board_after_move.clone_from(board);
            let their_turn = !board_after_move.apply_move(move_);

            let value = if their_turn {
                // opponent move: flip board, alpha, beta to their perspective and flip returned value to ours
                board_after_move.flip_board();
                -self.minimax(&board_after_move, remaining_depth - 1, -beta, -alpha, &mut search_line)
            } else {
                // bonus move: don't decrease depth
                self.minimax(&board_after_move, remaining_depth, alpha, beta, &mut search_line)
            }
            .increase_plies();

            if value >= best_value {
                best_value = value;
            }

            if value > beta {
                // beta cutoff, return early
                break;
            }

            if best_value > alpha {
                alpha = value;

                // we beat the current pv: overwrite (relative) pv with current line
                principal_line.overwrite(move_, &search_line);
            }
        }

        best_value
    }

    pub fn start_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        let mut current_best_value = Valuation::TerminalBlackWin { plies: 0 };

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        let mut pv = Line::new();

        let max_depth = 6;
        // {
        for max_depth in 1.. {
            if max_depth > LINE_MAX_SIZE as u32 {
                panic!(
                    "Tried searching to depth {}, but MOVE_LINE_MAX is {}",
                    max_depth, LINE_MAX_SIZE
                );
            }

            me.extend_pv(&board, &mut pv);

            let best_value = me.minimax(&board, max_depth, alpha, beta, &mut pv);

            if !me.search_state.lock().unwrap().search_active {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                    println!("* Best move had value {:?}", current_best_value);
                    println!("* NPS: {:.2e} ({:?})", me.current_nps(), me.start_t.elapsed());
                    println!("--------------------------------------------\n");
                }
                return;
            }

            if let Valuation::TerminalWhiteWin { plies } = best_value {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain win in {} plies", plies);
                    println!("--------------------------------------------\n");
                }
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.principal_variation = pv;
                    search_state.search_active = false;
                }
                return;
            }

            if let TerminalBlackWin { plies } = best_value {
                // all moves are certain losses, pick the one with the most plies and exit
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain loss in {} plies", plies);
                    println!("--------------------------------------------");
                    println!();
                }
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.principal_variation = pv;
                    search_state.search_active = false;
                }
                return;
            }

            me.search_state.lock().unwrap().principal_variation = pv;
            current_best_value = best_value;
        }

        me.search_state.lock().unwrap().search_active = false;

        if LOG_STATS {
            println!("--------------------------------------------");
            println!("* Minimax worker exited after search depth {}", max_depth);
            println!(
                "* Best move {} had value {:?}",
                me.search_state.lock().unwrap().principal_variation.best_move().unwrap(),
                current_best_value
            );
            println!("* NPS: {:.2e} ({:?})", me.current_nps(), me.start_t.elapsed());
            println!("--------------------------------------------\n");
        }
    }
}

/*====================================================================================================================*/

pub fn minimax_search(board: &Board, valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) {
    assert!(
        board.has_legal_move(),
        "Called minimax_search on board with no legal moves"
    );

    let t_handle;

    {
        // let worker_board = board.clone();

        t_handle = std::thread::spawn({
            let board = board.clone();
            move || {
                let worker: PVSWorker = PVSWorker::new(valuation_fn, search_state);
                worker.start_search(board);
            }
        });
    }

    // detach worker thread; will get shut down automatically when search_active gets set to false
    drop(t_handle);
}
