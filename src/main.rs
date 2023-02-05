mod agent;
mod kalah;
mod kgp;
mod minimax;
mod minimax_reference;
mod pvs;
mod tournament;
mod util;

pub use kalah::{Board, House, Move, Player};

use crate::kgp::Connection;

/*====================================================================================================================*/

pub const LOG_STATS: bool = true;

/*====================================================================================================================*/

/* fn single_ply<const DO_LOGGING: bool>(
    board: &mut Board,
    playing_agent: &mut impl Agent,
    player: Player,
    thinking_time: Duration,
) -> Player {
    if DO_LOGGING {
        println!("{board}\n");
    }

    match player {
        Player::White => playing_agent.update_board(board),
        Player::Black => {
            board.flip_board();
            playing_agent.update_board(board);
            board.flip_board();
        }
    };

    let start_time = std::time::Instant::now();

    playing_agent.go();

    let mut player_move = playing_agent.get_current_best_move();

    while playing_agent.get_state() == AgentState::Go
        && (playing_agent.is_reference() || start_time.elapsed() < thinking_time)
    {
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
        panic!("Invalid move {player_move:?} by Player {player} in position \n{board}\n\n");
        /* player_move = *valid_moves.choose(&mut thread_rng()).unwrap();
        println!("Invalid move, using {} instead", player_move); */
    }

    if DO_LOGGING {
        println!("{player}: playing move {player_move}");
    }

    let moves_again = board.apply_move(player_move);

    if DO_LOGGING {
        println!();
    }

    if moves_again {
        player
    } else {
        !player
    }
}

fn game_loop<const DO_LOGGING: bool>(
    board: Board,
    white_agent: impl Agent,
    black_agent: impl Agent,
    thinking_time: Duration,
) -> Board {
    use Player::{Black, White};

    let mut current_player = if !board.flipped() { White } else { Black };

    let mut board = board;
    let mut white_agent = white_agent;
    let mut black_agent = black_agent;

    loop {
        current_player = match current_player {
            White => single_ply::<DO_LOGGING>(&mut board, &mut white_agent, White, thinking_time),
            Black => single_ply::<DO_LOGGING>(&mut board, &mut black_agent, Black, thinking_time),
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
    let thinking_time = Duration::from_secs(3);

    let board = game_loop::<true>(board, white_agent, black_agent, thinking_time);

    println!("\nFinal board:\n\n{board}\n");

    match board.our_store.cmp(&board.their_store) {
        std::cmp::Ordering::Less => println!("Black won."),
        std::cmp::Ordering::Equal => println!("Draw."),
        std::cmp::Ordering::Greater => println!("White won."),
    }
} */

/* #[allow(dead_code)]
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
    assert_eq!(num_runs % 2, 0, "num_runs must be divisible by 2");

    // let num_workers = 8;
    let num_workers = num_cpus::get() / 2;

    let thinking_time = Duration::from_secs(2);

    println!("Running with {num_workers} workers");

    let agent1_white_wins = Arc::new(AtomicU64::new(0));
    let agent1_black_wins = Arc::new(AtomicU64::new(0));

    let agent2_white_wins = Arc::new(AtomicU64::new(0));
    let agent2_black_wins = Arc::new(AtomicU64::new(0));

    let draws = Arc::new(AtomicU64::new(0));

    let pool = ThreadPool::new(num_workers);

    // let mut t_handles = Vec::with_capacity(100);

    for _ in 0..num_runs / 2 {
        let mut board = Board::new(h, s);

        advance_random(h, s, &mut board, 2 * h as usize);

        // agent1 as White, agent2 as Black
        pool.execute({
            let board = board.clone();

            let agent1 = agent1_builder();
            let agent2 = agent2_builder();

            let agent1_white_wins = Arc::clone(&agent1_white_wins);
            let agent2_black_wins = Arc::clone(&agent2_black_wins);
            let draws = Arc::clone(&draws);

            move || {
                use std::cmp::Ordering::{Equal, Greater, Less};

                let board = game_loop::<false>(board, agent1, agent2, thinking_time);

                match board.our_store.cmp(&board.their_store) {
                    Less => agent2_black_wins.fetch_add(1, Ordering::Release),
                    Equal => draws.fetch_add(1, Ordering::Release),
                    Greater => agent1_white_wins.fetch_add(1, Ordering::Release),
                };
            }
        });

        // agent2 as White, agent1 as Black
        pool.execute({
            let board = board.clone();

            let agent1 = agent1_builder();
            let agent2 = agent2_builder();

            let agent1_black_wins = Arc::clone(&agent1_black_wins);
            let agent2_white_wins = Arc::clone(&agent2_white_wins);
            let draws = Arc::clone(&draws);

            move || {
                use std::cmp::Ordering::{Equal, Greater, Less};

                let board = game_loop::<false>(board, agent2, agent1, thinking_time);

                match board.our_store.cmp(&board.their_store) {
                    Less => agent1_black_wins.fetch_add(1, Ordering::Release),
                    Equal => draws.fetch_add(1, Ordering::Release),
                    Greater => agent2_white_wins.fetch_add(1, Ordering::Release),
                };
            }
        });
    }

    let mut num_done = 0;

    match num_runs {
        num_runs if num_runs < 10 => println!("{num_done:01}/{num_runs:01}"),
        num_runs if num_runs < 100 => println!("{num_done:02}/{num_runs:02}"),
        num_runs if num_runs < 1000 => println!("{num_done:03}/{num_runs:03}"),
        _ => panic!("formatting for {num_runs} num_runs not supported"),
    };

    loop {
        let queue_count = pool.queued_count();
        let active_count = pool.active_count();

        let new_num_done = num_runs - queue_count - active_count;

        if new_num_done > num_done {
            num_done = new_num_done;

            match num_runs {
                num_runs if num_runs < 10 => println!("{num_done:01}/{num_runs:01}"),
                num_runs if num_runs < 100 => println!("{num_done:02}/{num_runs:02}"),
                num_runs if num_runs < 1000 => println!("{num_done:03}/{num_runs:03}"),
                _ => panic!("formatting for {num_runs} num_runs not supported"),
            };
        }

        // if queue_count + active_count == 0 {
        if num_done == num_runs {
            break;
        }

        std::thread::sleep(Duration::from_secs(1));
    }

    pool.join();

    println!(
        "Agent 1 wins: {}/{}",
        agent1_white_wins.load(Ordering::Acquire),
        agent1_black_wins.load(Ordering::Acquire)
    );
    println!("Draws:        {}", draws.load(Ordering::Acquire));
    println!(
        "Agent 2 wins: {}/{}",
        agent2_white_wins.load(Ordering::Acquire),
        agent2_black_wins.load(Ordering::Acquire)
    );
} */

/* pub fn compare_agents(board: Board, mut agent1: impl Agent, mut agent2: impl Agent) {
    println!("{board}\n\n");

    agent1.update_board(&board);
    agent1.go();

    agent2.update_board(&board);
    agent2.go();

    // std::thread::sleep(Duration::from_secs(1));

    while agent1.get_state() == AgentState::Go {
        let _ = agent1.get_current_best_move();
        std::thread::sleep(Duration::from_millis(50));
    }

    while agent2.get_state() == AgentState::Go {
        let _ = agent2.get_current_best_move();
        std::thread::sleep(Duration::from_millis(50));
    }
} */

/* fn main() {
    let h = 8;
    let s = 8;

    // let white_agent = agent::RandomAgent::new(h, s);
    // let white_agent = minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::store_diff_valuation);
    let white_agent = pvs::PVSAgent::new(Board::new(h, s), kalah::valuation::store_diff_valuation);
    // let white_agent = agent::FirstMoveAgent::new(h, s);

    // let black_agent = agent::RandomAgent::new(h, s);
    let black_agent = minimax_reference::MinimaxAgent::new(Board::new(h, s), 6, kalah::valuation::store_diff_valuation);
    // let black_agent = agent::FirstMoveAgent::new(h, s);

    play_game(h, s, white_agent, black_agent);

    // let mut board = Board::new(h, s);
    // advance_random(h, s, &mut board, 2 * h as usize);
    // compare_agents(board, white_agent, black_agent);
} */

/* fn main() {
    let h = 8;
    let s = 8;

    let agent1_builder = &|| minimax::MinimaxAgent::new(Board::new(h, s), kalah::valuation::store_diff_valuation);

    // let agent2_builder = &|| agent::RandomAgent::new(h, s);
    // let agent2_builder =
    //     &|| minimax_reference::MinimaxAgent::new(Board::new(h, s), 6, kalah::valuation::store_diff_valuation);
    let agent2_builder = &|| pvs::PVSAgent::new(Board::new(h, s), kalah::valuation::store_diff_valuation);

    test_agents(h, s, agent1_builder, agent2_builder, 4 * 8);
} */

/* fn main() {
    let url = "wss://kalah.kwarc.info/socket";

    println!("Connecting to game server at {url}...");

    let conn = Connection::new_websocket(url).expect("Failed to connect");

    println!("Connected to game server {url}");

    crate::kgp::kgp_connect(conn);
} */

fn main() {
    let url = "localhost:2671";

    println!("Connecting to game server at {url}...");

    let conn = Connection::new_tcpstream(url).expect("Failed to connect");

    println!("Connected to game server {url}");

    crate::kgp::kgp_connect(conn);
}

/* fn generate_new_token() {
    let mut bytes: [u8; 64] = [0; 64];

    thread_rng().fill(&mut bytes);

    let token = base64::encode(bytes);

    println!("{token}");
} */
