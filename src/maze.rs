use std::fs::File;
use std::io::{BufRead, BufReader};

pub type Maze = Vec<Vec<char>>;

pub fn load_maze(path: &str) -> Maze {
    let file = File::open(path).expect("No se pudo abrir el archivo maze.txt");
    let reader = BufReader::new(file);

    let grid = reader
        .lines()
        .map(|line| line.expect("Error leyendo línea").chars().collect::<Vec<char>>())
        .collect();

    grid
}
