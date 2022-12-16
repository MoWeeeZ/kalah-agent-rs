use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

mod agent;
mod kalah;
mod kgp;
mod mcts;
mod minimax;
mod util;

use agent::Agent;
pub use kalah::{Board, Move, Player};
use kgp::{Command, Connection};
use threadpool::ThreadPool;

fn single_ply(
    board: &mut Board,
    playing_agent: &mut impl Agent,
    opponent_agent: &mut impl Agent,
    player: Player,
    print: bool,
) -> Player {
    if print {
        println!("{}\n", board);
    }

    let start_time = std::time::Instant::now();

    let mut player_move = playing_agent.get_move();

    let end_time = std::time::Instant::now();

    let dur = end_time - start_time;

    if print {
        println!("{} decided to make move {} after {:?}", player, player_move, dur);
    }

    if player == Player::Black {
        player_move = player_move.flip_player();
    }

    let valid_moves = board.legal_moves(player);

    if !valid_moves.iter().any(|valid_move| *valid_move == player_move) {
        panic!("Invalid move {} in position \n{}\n\n", player_move, board);
        /* player_move = *valid_moves.choose(&mut thread_rng()).unwrap();
        println!("Invalid move, using {} instead", player_move); */
    }

    if print {
        println!();
    }

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

fn game_loop(board: Board, white_agent: impl Agent, black_agent: impl Agent, print: bool) -> Board {
    use Player::{Black, White};

    let mut current_player = White;

    let mut board = board;
    let mut white_agent = white_agent;
    let mut black_agent = black_agent;

    loop {
        current_player = match current_player {
            White => single_ply(&mut board, &mut white_agent, &mut black_agent, White, print),
            Black => single_ply(&mut board, &mut black_agent, &mut white_agent, Black, print),
        };

        if !board.has_legal_move() {
            break;
        }
    }

    board
}

pub fn play_game<WhiteAgent, BlackAgent>(h: u8, s: u16, white_agent: WhiteAgent, black_agent: BlackAgent)
where
    WhiteAgent: Agent,
    BlackAgent: Agent,
{
    let board = Board::new(h, s);

    let board = game_loop(board, white_agent, black_agent, true);

    println!("\nFinal board:\n\n{}\n", board);

    match board.our_store.cmp(&board.their_store) {
        std::cmp::Ordering::Less => println!("Black won."),
        std::cmp::Ordering::Equal => println!("Draw."),
        std::cmp::Ordering::Greater => println!("White won."),
    }
}

pub fn test_agents<WhiteAgent, BlackAgent>(
    h: u8,
    s: u16,
    white_agent_builder: &dyn Fn() -> WhiteAgent,
    black_agent_builder: &dyn Fn() -> BlackAgent,
    num_runs: usize,
) where
    WhiteAgent: Agent + Send + 'static,
    BlackAgent: Agent + Send + 'static,
{
    let num_workers = 8;

    let white_wins = Arc::new(AtomicU64::new(0));
    let black_wins = Arc::new(AtomicU64::new(0));
    let draws = Arc::new(AtomicU64::new(0));

    let pool = ThreadPool::new(num_workers);

    // let mut t_handles = Vec::with_capacity(100);

    for _ in 0..num_runs {
        let board = Board::new(h, s);

        let white_agent = white_agent_builder();
        let black_agent = black_agent_builder();

        let white_wins = Arc::clone(&white_wins);
        let black_wins = Arc::clone(&black_wins);
        let draws = Arc::clone(&draws);

        // let board = game_loop(board, white_agent, black_agent, false);

        pool.execute(move || {
            let board = game_loop(board, white_agent, black_agent, false);

            match board.our_store.cmp(&board.their_store) {
                std::cmp::Ordering::Less => black_wins.fetch_add(1, Ordering::Release),
                std::cmp::Ordering::Equal => draws.fetch_add(1, Ordering::Release),
                std::cmp::Ordering::Greater => white_wins.fetch_add(1, Ordering::Release),
            };
        });
    }

    pool.join();

    println!("White wins: {}", white_wins.load(Ordering::Acquire));
    println!("Draws:      {}", draws.load(Ordering::Acquire));
    println!("Black wins: {}", black_wins.load(Ordering::Acquire));
}

fn kgp_connect(url: &str) {
    let mut conn = Connection::new(url).expect("Failed to connect");

    // let name = "Sauerkraut";
    // let authors = "";
    // let description = "";
    // let token = "";

    loop {
        let cmd: Command = match conn.read_command() {
            Ok(cmd) => cmd,
            Err(err) => match err {
                tungstenite::Error::Io(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                _ => {
                    eprintln!("Couldn't read command: {}", err);
                    return;
                }
            },
        };

        println!("{:?}", cmd);

        match cmd {
            Command::Kpg {
                id,
                ref_id: _,
                major,
                minor,
                patch,
            } => {
                if major != 1 {
                    conn.write_command("error protocol not supported", id);
                    eprintln!("Server tried to use unsupported protocol {}.{}.{}", major, minor, patch);
                    return;
                }

                // TODO: send server name, authoers and token
                // conn.write_command(&format!("set info:name {}", name), None);
                // conn.write_command(&format!("set info:authors {}", authors), None);
                // conn.write_command(&format!("set info:description {}", description), None);
                // conn.write_command(&format!("set info:token {}", token), None);
            }
            _ => todo!(),
        }
    }
}

fn main() {
    /* let h = 6;
    let s = 4;

    let thinking_duration = Duration::from_secs(1);

    let white_agent = minimax::MinimaxAgent::new(h, s, thinking_duration, true, kalah::valuation::seed_diff_valuation);
    // let white_agent = agent::MctsAgent::new(h, s, 2);

    // let black_agent = agent::RandomAgent::new(h, s);
    let black_agent = minimax::MinimaxAgent::new(h, s, thinking_duration, true, kalah::valuation::store_diff_valuation);

    play_game(h, s, white_agent, black_agent); */

    /* let url = "wss://kalah.kwarc.info/socket";

    let conn = match Connection::new(url) {
        Ok(conn) => conn,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    }; */

    let url = "wss://kalah.kwarc.info/socket";

    kgp_connect(url);
}
