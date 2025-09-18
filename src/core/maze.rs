//! Maze loading and normalization.
use std::fs::File;
use std::io::{BufRead, BufReader};

pub type Maze = Vec<Vec<char>>;

pub fn load_maze(path: &str) -> Maze {
    let file = File::open(path).expect("No pude abrir el maze.txt");
    let reader = BufReader::new(file);
    let mut grid: Maze = Vec::new();
    for line in reader.lines() {
        let mut row: Vec<char> = Vec::new();
        if let Ok(s) = line {
            for ch in s.chars() {
                if ch == ' ' || ch == 'g' || ch == '+' || ch == '-' || ch == '|' {
                    row.push(ch);
                } else {
                    if ch == '\t' { row.push(' ') } else { row.push('#') }
                }
            }
        }
        if !row.is_empty() { grid.push(row); }
    }
    let maxw = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    for r in &mut grid { while r.len() < maxw { r.push('#'); } }
    let mut has_exit = false;
    for row in &grid { if row.iter().any(|&c| c == 'g') { has_exit = true; break; } }
    if !has_exit {
        let mut best: Option<(usize,usize,usize)> = None;
        for (j,row) in grid.iter().enumerate() {
            for (i,&c) in row.iter().enumerate() {
                if c == ' ' { let d = i*i + j*j; if best.map(|b| d > b.2).unwrap_or(true) { best = Some((i,j,d)); } }
            }
        }
        if let Some((i,j,_)) = best { grid[j][i] = 'g'; }
    }
    grid
}
