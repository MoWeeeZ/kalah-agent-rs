use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

mod agent;
mod kalah;
mod kgp;
mod minimax;
mod minimax_move_ordering;
mod util;

use agent::Agent;
pub use kalah::{Board, Move, Player};
use kgp::{Command, Connection};
use threadpool::ThreadPool;

static THINKING_TIME: Duration = Duration::from_secs(1);

fn single_ply(board: &mut Board, playing_agent: &mut impl Agent, player: Player, print: bool) -> Player {
    if print {
        println!("{}\n", board);
    }

    match player {
        Player::White => playing_agent.set_board(board),
        Player::Black => {
            let mut board = board.clone();
            board.flip_board();
            playing_agent.set_board(&board)
        }
    };

    let start_time = std::time::Instant::now();

    playing_agent.go();

    let mut player_move = playing_agent.get_current_best_move();

    while start_time.elapsed() < THINKING_TIME {
        player_move = playing_agent.get_current_best_move();

        std::thread::sleep(Duration::from_millis(50));
    }

    playing_agent.stop();

    if player == Player::Black {
        // Black thinks they're White
        player_move = player_move.flip_player();
    }

    let valid_moves = board.legal_moves(player);

    if !valid_moves.iter().any(|valid_move| *valid_move == player_move) {
        panic!("Invalid move {} in position \n{}\n\n", player_move, board);
        /* player_move = *valid_moves.choose(&mut thread_rng()).unwrap();
        println!("Invalid move, using {} instead", player_move); */
    }

    let moves_again = board.apply_move(player_move);

    if print {
        println!();
    }

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
            White => single_ply(&mut board, &mut white_agent, White, print),
            Black => single_ply(&mut board, &mut black_agent, Black, print),
        };

        if !board.has_legal_move() {
            break;
        }
    }

    board
}

#[allow(dead_code)]
pub fn play_game<WhiteAgent, BlackAgent>(h: u8, s: u16, white_agent: WhiteAgent, black_agent: BlackAgent)
where
    WhiteAgent: Agent,
    BlackAgent: Agent,
{
    let board = Board::new(h, s);

    let board = game_loop(board, white_agent, black_agent, false);

    println!("\nFinal board:\n\n{}\n", board);

    match board.our_store.cmp(&board.their_store) {
        std::cmp::Ordering::Less => println!("Black won."),
        std::cmp::Ordering::Equal => println!("Draw."),
        std::cmp::Ordering::Greater => println!("White won."),
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    let h = 6;
    let s = 4;

    // let white_agent_builder = || minimax::MinimaxAgent::<true>::new(h, s, kalah::valuation::seed_diff_valuation);
    // let white_agent = agent::MctsAgent::new(h, s, 2);
    let white_agent = agent::FirstMoveAgent::new(h, s);

    // let black_agent_builder = || minimax_move_ordering::MinimaxAgent::<true>::new(h, s, kalah::valuation::seed_diff_valuation);
    // let black_agent = agent::RandomAgent::new(h, s);
    // let black_agent = minimax::MinimaxAgent::new(h, s, thinking_duration, true, kalah::valuation::store_diff_valuation);
    let black_agent = agent::FirstMoveAgent::new(h, s);

    let start_t = std::time::Instant::now();
    play_game(h, s, white_agent, black_agent);
    println!("Time: {:?}", start_t.elapsed());

    // test_agents(h, s, &white_agent_builder, &black_agent_builder, 20);

    /* let url = "wss://kalah.kwarc.info/socket";

    let conn = match Connection::new(url) {
        Ok(conn) => conn,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    }; */

    // let url = "wss://kalah.kwarc.info/socket";

    // kgp_connect(url);
}
