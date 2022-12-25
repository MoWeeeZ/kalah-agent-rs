use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

mod agent;
mod kalah;
mod kgp;
mod minimax;
mod util;

use agent::{Agent, AgentState};
pub use kalah::{Board, House, Move, Player};
use kgp::{Command, Connection};
use threadpool::ThreadPool;
use url::Url;

static THINKING_TIME: Duration = Duration::from_secs(1);

fn single_ply(board: &mut Board, playing_agent: &mut impl Agent, player: Player, print: bool) -> Player {
    if print {
        println!("{}\n", board);
    }

    match player {
        Player::White => playing_agent.update_board(board),
        Player::Black => {
            let mut board = board.clone();
            board.flip_board();
            playing_agent.update_board(&board)
        }
    };

    let start_time = std::time::Instant::now();

    playing_agent.go();

    let mut player_move = playing_agent.get_current_best_move();

    while start_time.elapsed() < THINKING_TIME && playing_agent.get_state() == AgentState::Go {
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
        panic!(
            "Invalid move {} by Player {} in position \n{}\n\n",
            player_move, player, board
        );
        /* player_move = *valid_moves.choose(&mut thread_rng()).unwrap();
        println!("Invalid move, using {} instead", player_move); */
    }

    if print {
        println!("{}: playing move {}", player, player_move);
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

    let board = game_loop(board, white_agent, black_agent, true);

    println!("\nFinal board:\n\n{}\n", board);

    match board.our_store.cmp(&board.their_store) {
        std::cmp::Ordering::Less => println!("Black won."),
        std::cmp::Ordering::Equal => println!("Draw."),
        std::cmp::Ordering::Greater => println!("White won."),
    }
}

#[allow(dead_code)]
pub fn test_agents<Agent1, Agent2>(
    h: u8,
    s: u16,
    agent1_builder: &dyn Fn() -> Agent1,
    agent2_builder: &dyn Fn() -> Agent2,
    num_runs: usize,
) where
    Agent1: Agent + Send + 'static,
    Agent2: Agent + Send + 'static,
{
    let num_workers = num_cpus::get() / 2;

    println!("Running with {} workers", num_workers);

    let agent1_wins = Arc::new(AtomicU64::new(0));
    let agent2_wins = Arc::new(AtomicU64::new(0));
    let draws = Arc::new(AtomicU64::new(0));

    let pool = ThreadPool::new(num_workers);

    // let mut t_handles = Vec::with_capacity(100);

    for i in 0..num_runs {
        // let board = game_loop(board, white_agent, black_agent, false);

        pool.execute({
            let board = Board::new(h, s);

            let agent1 = agent1_builder();
            let agent2 = agent2_builder();

            let agent1_wins = Arc::clone(&agent1_wins);
            let agent2_wins = Arc::clone(&agent2_wins);
            let draws = Arc::clone(&draws);

            move || {
                if i % 2 == 0 {
                    let board = game_loop(board, agent1, agent2, false);

                    match board.our_store.cmp(&board.their_store) {
                        std::cmp::Ordering::Less => agent2_wins.fetch_add(1, Ordering::Release),
                        std::cmp::Ordering::Equal => draws.fetch_add(1, Ordering::Release),
                        std::cmp::Ordering::Greater => agent1_wins.fetch_add(1, Ordering::Release),
                    };
                } else {
                    let board = game_loop(board, agent2, agent1, false);

                    match board.our_store.cmp(&board.their_store) {
                        std::cmp::Ordering::Less => agent1_wins.fetch_add(1, Ordering::Release),
                        std::cmp::Ordering::Equal => draws.fetch_add(1, Ordering::Release),
                        std::cmp::Ordering::Greater => agent2_wins.fetch_add(1, Ordering::Release),
                    };
                };
            }
        });
    }

    // pool.join();

    let mut num_done = 0;

    println!("{:02}/{:02}", num_done, num_runs);

    loop {
        let queue_count = pool.queued_count();
        let active_count = pool.active_count();

        let new_num_done = num_runs - queue_count - active_count;

        if new_num_done > num_done {
            num_done = new_num_done;
            println!("{:02}/{:02}", num_done, num_runs);
        }

        // if queue_count + active_count == 0 {
        if num_done == num_runs {
            break;
        }

        std::thread::sleep(Duration::from_secs(1));
    }

    pool.join();

    println!("Agent 1 wins: {}", agent1_wins.load(Ordering::Acquire));
    println!("Draws:        {}", draws.load(Ordering::Acquire));
    println!("Agent 2 wins: {}", agent2_wins.load(Ordering::Acquire));
}

fn process_command(conn: &mut Connection, active_agents: &mut HashMap<u32, (Box<dyn Agent>, Option<Move>)>) {
    let new_agent = |board: Board| {
        Box::new(minimax::MinimaxAgent::new(
            board,
            kalah::valuation::store_diff_valuation,
        ))
    };

    let cmd = match conn.read_command() {
        Some(cmd) => cmd,
        None => return,
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
                std::process::exit(1);
            }

            // TODO: send server name, authors and token
            // conn.write_command(&format!("set info:name {}", name), None);
            // conn.write_command(&format!("set info:authors {}", authors), None);
            // conn.write_command(&format!("set info:description {}", description), None);
            // conn.write_command(&format!("set info:token {}", token), None);

            conn.write_command("mode freeplay", None);

            println!("Selected mode: freeplay");
        }
        Command::State { id, ref_id, board } => {
            let id = id.expect("Server didn't attach id to state");

            if id > 50 && board.our_store < 5 && board.their_store < 5 {
                // server trying to start second game
                std::process::exit(0);
            }

            if active_agents.contains_key(&id) {
                // Duplicate IDs by the server are ignored
                return;
            }

            let mut agent = if let Some(ref_id) = ref_id {
                active_agents
                    .remove(&ref_id)
                    .expect("Server referenced ID that didn't exist")
                    .0
            } else {
                new_agent(board)
            };

            agent.go();

            // insert agent with no current best move
            active_agents.insert(id, (agent, None));
        }
        Command::Stop { id: _id, ref_id } => {
            let ref_id = ref_id.unwrap();
            let (mut agent, best_move) = active_agents.remove(&ref_id).unwrap();
            println!("{} best move: {}", ref_id, best_move.unwrap().house() + 1);
            agent.stop();
        }
        Command::Ok { .. } => { /* ignore */ }
        Command::Error { id: _, ref_id: _, msg } => {
            eprintln!("error {}", msg);
            std::process::exit(1);
        }
        Command::Ping { id, ref_id: _, msg } => {
            conn.write_command(&format!("pong {}", msg), id);
        }
        Command::Pong { .. } => { /* ignore */ }
        Command::Goodbye { .. } => {
            std::process::exit(0);
        }
    }
}

#[allow(dead_code)]
fn kgp_connect(url: &Url) {
    println!("Connecting to game server {}...", url);

    let mut conn = Connection::new(url).expect("Failed to connect");

    println!("Connected to game server {}", url);

    // map of agents and their last best move
    let mut active_agents: HashMap<u32, (Box<dyn Agent>, Option<Move>)> = HashMap::new();

    // let name = "Sauerkraut";
    // let authors = "";
    // let description = "";

    /* let token = match std::fs::read("TOKEN") {
        Ok(raw_content) => String::from_utf8(raw_content).unwrap(),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                eprintln!("Not TOKEN file found");
                "".to_owned()
            } else {
                panic!("{}", err)
            }
        }
    }; */

    loop {
        process_command(&mut conn, &mut active_agents);

        for (&id, (agent, last_best_move)) in active_agents.iter_mut() {
            let best_move = agent.get_current_best_move();

            if &Some(best_move) == last_best_move {
                continue;
            }

            conn.write_command(&format!("move {}", best_move.house() + 1), Some(id));

            *last_best_move = Some(best_move);
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

fn main() {
    /* let h = 8;
    let s = 8; */

    /* // let white_agent = agent::RandomAgent::new(h, s);
    let white_agent = minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::seed_diff_valuation);
    // let white_agent = agent::FirstMoveAgent::new(h, s);

    let black_agent = agent::RandomAgent::new(h, s);
    // let black_agent = minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::store_diff_valuation);
    // let black_agent = agent::FirstMoveAgent::new(h, s);

    // let start_t = std::time::Instant::now();
    play_game(h, s, white_agent, black_agent);
    // println!("{:?}", start_t.elapsed()); */

    /* let white_agent_builder = &|| minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::seed_diff_valuation);

    // let black_agent_builder = &|| agent::RandomAgent::new(h, s);
    let black_agent_builder = &|| minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::seed_diff_valuation);

    test_agents(h, s, white_agent_builder, black_agent_builder, 4 * 8); */

    let url: Url = "wss://kalah.kwarc.info/socket".parse().unwrap();
    // url.set_port(Some(2671)).unwrap();

    kgp_connect(&url);
}
