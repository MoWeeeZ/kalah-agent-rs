pub mod math;

/* pub fn advance_random(h: u8, s: u16, board: &mut Board, num_moves: usize) {
    let mut current_player = Player::White;
    let mut random_agent = RandomAgent::new(h, s);

    // since it uses RandomAgent moves should be basically instant anyways
    let thinking_duration = Duration::from_secs(1);

    // make 10 random moves
    for _ in 0..num_moves {
        use Player::{Black, White};

        current_player = match current_player {
            White => single_ply::<false>(board, &mut random_agent, White, thinking_duration),
            Black => single_ply::<false>(board, &mut random_agent, Black, thinking_duration),
        };

        if !board.has_legal_move() {
            break;
        }
    }
} */
