use std::collections::HashMap;

use crate::vector::Vec2isize;

pub struct Simulation {
    pub boards: Vec<Board>
}

pub struct Board {
    pub uid: u128,
    pub wires: Vec<Wire>,
    pub circuits: Vec<CircuitSavestate>,
    pub states: Vec<BoardStateSavestate>,
}

pub struct Wire {
    pub id: usize,
    pub points: Vec<(Vec2isize, [bool; 4])>
}