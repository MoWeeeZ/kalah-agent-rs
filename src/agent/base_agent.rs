use crate::board::Move;

pub trait Agent {
    fn inform_move(&mut self, move_: Move);
    fn get_move(&mut self) -> Move;
}
