use std::time::Duration;

use url::Url;

use crate::agent::{Agent, AgentState};
// use crate::kalah::valuation;
use crate::kgp::Connection;
use crate::tournament::MinimaxAgent;
use crate::Board;

use super::Command;

/*====================================================================================================================*/

/* #[derive(PartialEq, Eq)]
enum CtrlCStatus {
    Run,
    ExitAfterGame,
}

static mut CTRLC_STATUS: CtrlCStatus = CtrlCStatus::Run; */

/*====================================================================================================================*/

fn process_command(conn: &mut Connection, agent: &mut Box<dyn Agent>, cur_id: &mut u32) {
    // active_agents: &mut HashMap<u32, (Box<dyn Agent>, Option<Move>)>
    // let new_agent = |board: Board| Box::new(MinimaxAgent::new(board, valuation::store_diff_valuation));

    let cmd = match conn.read_command() {
        Some(cmd) => cmd,
        None => return,
    };

    // println!("{:?}", cmd);

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
                eprintln!("Server tried to use unsupported protocol {major}.{minor}.{patch}");
                std::process::exit(1);
            }

            let name = "Sauerkraut";
            // let authors = "Moritz Gmeiner";
            // let description = "Minimax with alpha-beta pruning";

            let token_path = std::env::var("TOKEN_PATH").unwrap_or_else(|_| "./TOKEN".to_owned());

            let token = match std::fs::read(token_path) {
                Ok(raw_content) => String::from_utf8(raw_content).unwrap(),
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        eprintln!("No TOKEN file found");
                        "".to_owned()
                    } else {
                        panic!("{}", err)
                    }
                }
            };

            // send server name, authors and token
            conn.write_command(&format!("set info:name {name}"), None);
            println!("Setting name: {name}");
            // conn.write_command(&format!("set info:authors {}", authors), None);
            // conn.write_command(&format!("set info:description {}", description), None);
            conn.write_command(&format!("set auth:token {token}"), None);
            // println!("Setting token: {}", token);

            conn.write_command("mode freeplay", None);

            println!("Selected mode: freeplay");
        }
        Command::State { id, ref_id, board } => {
            let id = id.expect("Server didn't attach id to state");

            /* if unsafe { CTRLC_STATUS == CtrlCStatus::ExitAfterGame } && board.our_store < 5 && board.their_store < 5 {
                // server trying to start second game
                println!("Game finished, exiting");
                std::process::exit(0);
            } */

            println!("\n\n{board}\n");

            if let Some(ref_id) = ref_id {
                assert_eq!(
                    ref_id, *cur_id,
                    "Server referenced ID {ref_id}, but current ID is {cur_id}"
                );
            }

            agent.update_board(&board);
            *cur_id = id;

            agent.go();
            println!("go");
        }
        Command::Stop { id: _id, ref_id } => {
            let ref_id = ref_id.unwrap();
            assert_eq!(
                ref_id, *cur_id,
                "Server told ID {ref_id} to stop, but current ID is {cur_id}"
            );
            // let (mut agent, best_move) = active_agents.remove(&ref_id).unwrap();
            println!("{ref_id} stop");
            agent.stop();
        }
        Command::Ok { .. } => {
            println!("ok");
        }
        Command::Set {
            id: _id,
            ref_id: _ref_id,
            option,
            value,
        } => {
            println!("server set {option} to {value}");
        }
        Command::Error { id: _, ref_id: _, msg } => {
            eprintln!("ERROR {msg}");
            std::process::exit(1);
        }
        Command::Ping { id, ref_id: _, msg } => {
            conn.write_command(&format!("pong {msg}"), id);
        }
        Command::Pong { .. } => { /* ignore */ }
        Command::Goodbye { .. } => {
            std::process::exit(0);
        }
    }
}

#[allow(dead_code)]
pub fn kgp_connect(url: &Url) {
    println!("Connecting to game server at {url}...");

    let mut conn = Connection::new(url).expect("Failed to connect");

    println!("Connected to game server {url}");

    /* ctrlc::set_handler(|| unsafe {
        match CTRLC_STATUS {
            CtrlCStatus::Run => {
                println!("Received Ctrl-C, exiting after game");
                CTRLC_STATUS = CtrlCStatus::ExitAfterGame;
            }
            CtrlCStatus::ExitAfterGame => {
                println!("Received Ctrl-C twice, exiting now");
                std::process::exit(0);
            }
        }
    })
    .expect("Could not set CtrlC handler"); */

    // map of agents and their last best move
    // let mut active_agents: HashMap<u32, (Box<dyn Agent>, Option<Move>)> = HashMap::new();
    let mut agent: Box<dyn Agent> = Box::new(MinimaxAgent::new(Board::new(8, 8)));
    let mut last_best_move = None;
    let mut id = 0;

    loop {
        process_command(&mut conn, &mut agent, &mut id);

        // for (&id, (agent, last_best_move)) in active_agents.iter_mut() {
        if agent.get_state() == AgentState::Waiting {
            continue;
        }

        let best_move = agent.get_current_best_move();

        if Some(best_move) == last_best_move {
            continue;
        }

        conn.write_command(&format!("move {}", best_move.house() + 1), Some(id));

        last_best_move = Some(best_move);
        // }

        std::thread::sleep(Duration::from_millis(50));
    }
}
