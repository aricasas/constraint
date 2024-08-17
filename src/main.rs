use constraint::sudoku::{self, Sudoku};

fn main() {
    // let mut sudoku = Sudoku::new();
    // sudoku.add_num(1, 0, 0);
    // sudoku.add_num(2, 1, 1);

    let sudoku = Sudoku::from_slice(&[
        3, 0, 6, 5, 0, 8, 4, 0, 0, 5, 2, 0, 0, 0, 0, 0, 0, 0, 0, 8, 7, 0, 0, 0, 0, 3, 1, 0, 0, 3,
        0, 1, 0, 0, 8, 0, 9, 0, 0, 8, 6, 3, 0, 0, 5, 0, 5, 0, 0, 9, 0, 6, 0, 0, 1, 3, 0, 0, 0, 0,
        2, 5, 0, 0, 0, 0, 0, 0, 0, 0, 7, 4, 0, 0, 5, 2, 0, 6, 3, 0, 0,
    ]);

    // let sudoku = Sudoku::from_slice(&[
    //     3, 1, 6, 5, 7, 8, 4, 9, 2, 5, 2, 9, 1, 3, 4, 7, 6, 8, 4, 8, 7, 6, 2, 9, 5, 3, 1, 2, 6, 3,
    //     0, 1, 5, 9, 8, 7, 9, 7, 4, 8, 6, 0, 1, 2, 5, 8, 5, 1, 7, 9, 2, 6, 4, 3, 1, 3, 8, 0, 4, 7,
    //     2, 0, 6, 6, 9, 2, 3, 5, 1, 8, 7, 4, 7, 4, 5, 0, 8, 6, 3, 1, 0,
    // ]);

    // let sudoku = Sudoku::from_slice(&[1; 81]);

    println!("{}", sudoku);

    let problem = sudoku.to_constraint_problem();
    println!("{:?}", problem);

    let problem = problem.normalize_problem();
    println!("NORMALIZED");

    let problem = problem.constraint_propagation().unwrap();
    println!("CONSTRAINTS");

    // let solution = problem.solve_backtracking().unwrap();
    let solution = problem.solve_cbj().unwrap();
    println!("SOLVED");
    let solution_board =
        Sudoku::from_slice(&solution.iter().map(|&x| x as u8).collect::<Vec<u8>>());
    println!("{}", solution_board);
}
