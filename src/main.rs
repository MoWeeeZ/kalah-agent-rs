use std::time::Duration;

use agent::{Agent, MinimaxAgent, RandomAgent};
use board::Player;
use rand::seq::SliceRandom;
use rand::thread_rng;

mod agent;
mod board;
mod mcts;
mod minimax;
mod util;

pub use board::{Board, Move};

fn single_ply(
    board: &mut Board,
    playing_agent: &mut impl Agent,
    opponent_agent: &mut impl Agent,
    player: Player,
) -> Player {
    println!("{}\n", board);

    let start_time = std::time::Instant::now();

    let mut player_move = playing_agent.get_move();

    let end_time = std::time::Instant::now();

    let dur = end_time - start_time;

    println!("{} decided to make move {} after {:?}", player, player_move, dur);

    if player == Player::Black {
        player_move = player_move.flip_player();
    }

    let valid_moves = board.legal_moves(player);

    if !valid_moves.iter().any(|valid_move| *valid_move == player_move) {
        player_move = *valid_moves.choose(&mut thread_rng()).unwrap();
        println!("Invalid move, using {} instead", player_move);
    }

    println!();

    let moves_again = match player {
        Player::White => {
            playing_agent.inform_move(player_move);
            opponent_agent.inform_move(player_move.flip_player());
            board.apply_move(player_move)
        }
        Player::Black => {
            playing_agent.inform_move(player_move.flip_player());
            opponent_agent.inform_move(player_move);
            board.apply_move(player_move)
        }
    };

    if moves_again {
        player
    } else {
        !player
    }
}

fn game_loop(board: Board, white_agent: impl Agent, black_agent: impl Agent) {
    let mut board = board;
    let mut white_agent = white_agent;
    let mut black_agent = black_agent;

    use Player::{Black, White};

    let mut current_player = White;

    loop {
        current_player = match current_player {
            White => single_ply(&mut board, &mut white_agent, &mut black_agent, White),
            Black => single_ply(&mut board, &mut black_agent, &mut white_agent, Black),
        };

        if !board.has_legal_move() {
            break;
        }
    }

    board.finish_game();

    println!("\nFinal board:\n\n{}\n", board);

    match board.our_store.cmp(&board.their_store) {
        std::cmp::Ordering::Less => println!("Black won."),
        std::cmp::Ordering::Equal => println!("Draw."),
        std::cmp::Ordering::Greater => println!("White won."),
    }
}

fn main() {
    let h = 6;
    let s = 4;

    let thinking_duration = Duration::from_secs(1);

    let board = Board::new(h, s);

    let white_agent = MinimaxAgent::new(h, s, thinking_duration);
    // let white_agent = MctsAgent::new(h, s, 2);

    let black_agent = RandomAgent::new(h, s);

    game_loop(board, white_agent, black_agent);
}
