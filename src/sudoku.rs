use std::fmt::Display;

use crate::{RawProblem, Variable};

pub struct Sudoku {
    board: [u8; 81],
}
impl Sudoku {
    pub fn new() -> Self {
        Self { board: [0; 81] }
    }
    pub fn from_slice(solution: &[u8]) -> Self {
        let mut board = [0; 81];
        board.copy_from_slice(&solution[0..81]);
        Self { board }
    }
    pub fn add_num(&mut self, val: u8, x: usize, y: usize) {
        self.board[9 * y + x] = val
    }
    pub fn to_constraint_problem(&self) -> RawProblem {
        let mut problem = RawProblem::new();

        for _ in 0..81 {
            problem.add_var(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        }

        let check_nine_distinct = |vals: &mut dyn Iterator<Item = i32>| {
            let mut bit_arr: u16 = 0;
            for val in vals {
                bit_arr |= 1 << val;
            }
            bit_arr == 0b1111111110
        };

        // No repeating in rows
        for y in 0..9 {
            let row = (0..9).map(|x| Variable { id: 9 * y + x }).collect();
            problem.add_constraint(row, Box::new(check_nine_distinct));
        }
        // No repeating in columns
        // for x in 0..9 {
        //     let column = (0..9).map(|y| Variable { id: 9 * y + x }).collect();
        //     problem.add_constraint(column, Box::new(check_nine_distinct));
        // }

        // No repeating in 3x3 squares
        // for sy in 0..3 {
        //     for sx in 0..3 {
        //         let top_left = 9 * 3 * sy + 3 * sx;
        //         let square = [
        //             top_left,
        //             top_left + 1,
        //             top_left + 2,
        //             top_left + 9,
        //             top_left + 9 + 1,
        //             top_left + 9 + 2,
        //             top_left + 18,
        //             top_left + 18 + 1,
        //             top_left + 18 + 2,
        //         ]
        //         .into_iter()
        //         .map(|i| Variable { id: i })
        //         .collect();

        //         problem.add_constraint(square, Box::new(check_nine_distinct));
        //     }
        // }

        // Tiles that are set must use those values
        for (i, &num) in self.board.iter().enumerate() {
            if num != 0 {
                problem.add_constraint(
                    vec![Variable { id: i }],
                    Box::new(move |vals| vals.next().unwrap() == num.into()),
                );
            }
        }

        problem
    }
}

fn check_distinct(array: &[i32]) -> bool {
    for i in 0..array.len() {
        for j in 0..i {
            if array[i] == array[j] {
                return false;
            }
        }
    }
    true
}

impl Display for Sudoku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..81 {
            write!(f, "{}", self.board[i])?;
            if i % 9 == 8 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let sudoku = Sudoku::new();
        println!("{}", sudoku);

        let problem = sudoku.to_constraint_problem();
        println!("{:?}", problem);

        let problem = problem
            .normalize_problem()
            .constraint_propagation()
            .unwrap();

        let solution = problem.solve_backtracking().unwrap();
        let solution_board =
            Sudoku::from_slice(&solution.iter().map(|&x| x as u8).collect::<Vec<u8>>());
        println!("{}", solution_board);
    }
}
