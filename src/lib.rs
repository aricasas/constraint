use hashbrown::{HashMap, HashSet};
use std::{cmp::Ordering, fmt::Debug};

pub mod sudoku;

type Universe = i32;
type Evaluation = Box<dyn Fn(&mut dyn Iterator<Item = Universe>) -> bool>;
type Candidate = Vec<Option<Universe>>;

pub struct Constraint {
    pub scope: Vec<Variable>,
    pub evaluate: Evaluation,
}
impl Debug for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Constraint")
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Domain {
    pub of: Variable,
    pub values: Vec<Universe>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Variable {
    pub id: usize,
}

#[derive(Debug)]
pub struct RawProblem {
    variables: Vec<Variable>,
    domains: Vec<Domain>,
    constraints: Vec<Constraint>,
}

impl RawProblem {
    pub fn new() -> Self {
        RawProblem {
            variables: Vec::new(),
            domains: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn add_var(&mut self, domain: Vec<Universe>) -> Variable {
        let new_var = Variable {
            id: self.variables.len(),
        };
        let new_domain = Domain {
            of: new_var,
            values: domain,
        };

        self.variables.push(new_var);
        self.domains.push(new_domain);

        new_var
    }

    pub fn add_constraint(&mut self, scope: Vec<Variable>, evaluation: Evaluation) {
        assert!(scope.is_sorted_by_key(|v| v.id));

        self.constraints.push(Constraint {
            scope,
            evaluate: evaluation,
        });
    }

    pub fn normalize_problem(self) -> NormalizedProblem {
        let mut normalized_cons: HashMap<Vec<Variable>, Evaluation> = HashMap::new();

        // Combine constraints with same scope
        for Constraint { scope, evaluate } in self.constraints {
            if let Some(curr_eval) = normalized_cons.remove(&scope) {
                normalized_cons.insert(scope, Box::new(move |u| curr_eval(u) && evaluate(u)));
            } else {
                normalized_cons.insert(scope, evaluate);
            }
        }

        NormalizedProblem {
            variables: self.variables,
            domains: self.domains,
            constraints: normalized_cons,
        }
    }
}
impl Default for RawProblem {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NormalizedProblem {
    pub variables: Vec<Variable>,
    pub domains: Vec<Domain>,
    pub constraints: HashMap<Vec<Variable>, Evaluation>,
}

impl NormalizedProblem {
    pub fn constraint_propagation(self) -> Option<PropagatedProblem> {
        self.make_node_consistency()
            .make_arc_consistency()
            .map(Self::sort_domains)
            .map(
                |NormalizedProblem {
                     variables,
                     domains,
                     constraints,
                 }| PropagatedProblem {
                    variables,
                    domains,
                    constraints: Self::sort_constraints(constraints.into_iter().collect()),
                },
            )
    }

    fn make_node_consistency(mut self) -> Self {
        for i in 0..self.variables.len() {
            let var = self.variables[i];
            let domain = &mut self.domains[i].values;

            if let Some(eval) = self.constraints.remove(&vec![var]) {
                domain.retain(|&vx| eval(&mut [vx].into_iter()));
            }
        }

        self
    }
    fn make_arc_consistency(mut self) -> Option<Self> {
        // Using AC-3 from https://en.wikipedia.org/wiki/AC-3_algorithm
        let mut vars_cartesian_product =
            Vec::with_capacity(self.variables.len() * self.variables.len());
        for &var1 in &self.variables {
            for &var2 in &self.variables {
                vars_cartesian_product.push((var1, var2));
            }
        }

        let mut worklist: Vec<(Variable, Variable)> =
            Vec::from_iter(vars_cartesian_product.iter().cloned().filter(|&(x, y)| {
                self.constraints.get(&vec![x, y]).is_some()
                    || self.constraints.get(&vec![y, x]).is_some()
            }));

        while let Some(arc) = worklist.pop() {
            let (x, y) = arc;

            if self.arc_reduce(x, y) {
                if self.domains[x.id].values.is_empty() {
                    return None;
                } else {
                    worklist.extend(vars_cartesian_product.iter().cloned().filter(|&(z, xx)| {
                        z != y && xx == x && self.constraints.get(&vec![z, x]).is_some()
                            || self.constraints.get(&vec![x, z]).is_some()
                    }))
                }
            }
        }

        Some(self)
    }
    fn arc_reduce(&mut self, x: Variable, y: Variable) -> bool {
        let mut change = false;

        for vx in self.domains[x.id].values.clone() {
            if !self.domains[y.id].values.iter().any(|&vy| {
                self.constraints
                    .get(&vec![x, y])
                    .is_some_and(|eval| eval(&mut [vx, vy].into_iter()))
            }) {
                self.domains[x.id].values.retain(|&vxx| vxx != vx);
                change = true;
            }
        }
        change
    }
    fn sort_domains(mut self) -> Self {
        for domain in self.domains.iter_mut() {
            domain.values.sort_unstable()
        }
        self
    }
    fn sort_constraints(
        mut constraints: Vec<(Vec<Variable>, Evaluation)>,
    ) -> Vec<(Vec<Variable>, Evaluation)> {
        constraints.sort_unstable_by(|(scope_a, _), (scope_b, _)| {
            let mut rev_a = scope_a.iter().rev();
            let mut rev_b = scope_b.iter().rev();
            loop {
                let a = rev_a.next().map(|v| v.id);
                let b = rev_b.next().map(|v| v.id);

                match (a, b) {
                    (None, None) => return Ordering::Equal,
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(a), Some(b)) => {
                        if a == b {
                            continue;
                        } else {
                            return a.cmp(&b);
                        }
                    }
                }
            }
        });
        constraints
    }
}

pub struct PropagatedProblem {
    pub variables: Vec<Variable>,
    pub domains: Vec<Domain>,
    pub constraints: Vec<(Vec<Variable>, Evaluation)>,
}

// Based on https://en.wikipedia.org/wiki/Backtracking and https://www.geeksforgeeks.org/sudoku-backtracking-7/
impl PropagatedProblem {
    pub fn solve_backtracking(&self) -> Option<Vec<Universe>> {
        let mut candidate: Candidate = vec![None; self.variables.len()];
        if self.backtrack(&mut candidate, 0) {
            candidate.into_iter().collect()
        } else {
            None
        }
    }
    fn backtrack(&self, candidate: &mut Candidate, k: usize) -> bool {
        // for _ in 0..k {
        //     print!("-");
        // }
        // println!();

        if self.reject(candidate, k) {
            return false;
        }
        if self.accept(candidate) {
            return true;
        }

        let mut s = self.first(candidate, k);
        while s {
            let res = self.backtrack(candidate, k + 1);
            if res {
                return true;
            }

            s = self.next(candidate, k + 1);
        }

        candidate[k] = None;
        false
    }
    /// Returns true if candidate values are inconsistent with constraints
    fn reject(&self, candidate: &Candidate, k: usize) -> bool {
        // let k = candidate.len();
        if k == 0 {
            return false;
        }

        let curr_var = self.variables[k - 1];

        let to_check = self
            .constraints
            .iter()
            .filter(|constraint| constraint.0.last() == Some(&curr_var));

        for constraint in to_check {
            let mut vals_needed = constraint.0.iter().map(|var| candidate[var.id].unwrap());
            if !constraint.1(&mut vals_needed) {
                return true;
            }
        }

        false
    }
    /// Returns true if candidate values are consistent and complete with constraints
    fn accept(&self, candidate: &Candidate) -> bool {
        candidate[candidate.len() - 1].is_some()
    }
    fn first(&self, candidate: &mut Candidate, k: usize) -> bool {
        // let k = candidate.len();
        if candidate.last().is_some_and(|x| x.is_some()) {
            false
        } else {
            let first_val_next_var = self.domains[k].values[0];
            // let mut next_cand = candidate.clone();
            // next_cand.push(first_val_next_var);
            // Some(next_cand)
            candidate[k] = Some(first_val_next_var);

            true
        }
    }
    fn next(&self, candidate: &mut Candidate, k: usize) -> bool {
        // let k = candidate.len();
        if candidate[k - 1] == self.domains[k - 1].values.last().copied() {
            return false;
        }

        let curr_val = candidate[k - 1].unwrap();
        let i = self.domains[k - 1].values.binary_search(&curr_val).unwrap();
        candidate[k - 1] = Some(self.domains[k - 1].values[i + 1]);
        true
    }
}

// CBJ based on https://cse.unl.edu/~choueiry/Documents/Hybrid-Prosser.pdf
// (HYBRID ALGORITHMS FOR THE CONSTRAINT SATISFACTION PROBLEM PATRICK PROSS)
// impl PropagatedProblem {
//     pub fn solve_cbj(&mut self) -> Option<Vec<Universe>> {
//         let mut vals = vec![0; self.variables.len()];
//         let mut current_domain = self.domains.iter().map(|dom| dom.values.clone()).collect();
//         let mut conf_set: Vec<HashSet<usize>> = vec![HashSet::new(); self.variables.len()];
//         let mut status = Status::Unknown;

//         self.cbj_bcssp(&mut vals, &mut current_domain, &mut conf_set, &mut status);

//         if status == Status::Solution {
//             Some(vals)
//         } else {
//             None
//         }
//     }

//     fn cbj_bcssp(
//         &mut self,
//         vals: &mut Vec<Universe>,
//         current_domain: &mut Vec<Vec<Universe>>,
//         conf_set: &mut Vec<HashSet<usize>>,
//         status: &mut Status,
//     ) {
//         let mut consistent = true;
//         *status = Status::Unknown;
//         let mut i = 0;
//         let n = self.variables.len();

//         while *status == Status::Unknown {
//             if consistent {
//                 i = self.cbj_label(i, vals, current_domain, conf_set, &mut consistent);
//             } else {
//                 i = self.cbj_unlabel(i, &mut consistent);
//             }
//             if i >= n {
//                 *status = Status::Solution;
//             } else if i == 0 {
//                 *status = Status::Impossible;
//             }
//         }
//     }
//     fn cbj_label(
//         &self,
//         i: usize,
//         vals: &mut Vec<Universe>,
//         current_domain: &mut Vec<Vec<Universe>>,
//         conf_set: &mut Vec<HashSet<usize>>,
//         consistent: &mut bool,
//     ) -> usize {
//         *consistent = false;
//         for &val in &current_domain[i] {
//             vals[i] = val;
//         }

//         todo!()
//     }
//     fn cbj_unlabel(&self, i: usize, consistent: &mut bool) -> usize {}
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// enum Status {
//     Unknown,
//     Solution,
//     Impossible,
// }

// Based on https://ics.uci.edu/~dechter/books/chapter06.pdf figure 6.7
impl PropagatedProblem {
    pub fn solve_cbj(&self) -> Option<Vec<Universe>> {
        let mut i: usize = 0;
        let n = self.variables.len();
        let mut curr_domain: Vec<Vec<Universe>> =
            self.domains.iter().map(|dom| dom.values.clone()).collect();
        let mut conf_set: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        let mut vals: Candidate = vec![None; n];

        while i < n {
            vals[i] = self.select_val_cbj(i, &mut curr_domain, &mut conf_set, &mut vals);

            if vals[i].is_none() {
                let i_prev = i;
                let max = conf_set[i].iter().max();
                if let Some(&max) = max {
                    i = max;
                    let b = conf_set[i_prev].clone();
                    conf_set[i].extend(&b);
                    conf_set[i].remove(&i);
                } else {
                    return None;
                }
            } else {
                i += 1;
                if i == n {
                    break;
                }
                self.domains[i].values.clone_into(&mut curr_domain[i]);
                conf_set[i].clear();
            }
        }

        vals.into_iter().collect()
    }

    fn select_val_cbj(
        &self,
        i: usize,
        curr_domain: &mut [Vec<Universe>],
        conf_set: &mut [HashSet<usize>],
        vals: &mut Candidate,
    ) -> Option<Universe> {
        while let Some(a) = curr_domain[i].pop() {
            vals[i] = Some(a);
            let mut consistent = true;
            let mut k = 0;
            while k < i && consistent {
                let broken_constraint = self.search_broken_constraint(i, k, vals);

                if broken_constraint.is_none() {
                    // Passed all consistency checks
                    k += 1;
                } else {
                    let scope = broken_constraint.unwrap();
                    conf_set[i].extend(scope.iter().filter_map(|var| {
                        if var.id != i {
                            Some(var.id)
                        } else {
                            None
                        }
                    }));
                    consistent = false;
                }
            }
            if consistent {
                return Some(a);
            }
        }

        None
    }

    fn search_broken_constraint(
        &self,
        i: usize,
        k: usize,
        vals: &Candidate,
    ) -> Option<&Vec<Variable>> {
        let mut broken_constraint = None;
        for (scope, eval) in &self.constraints {
            let len = scope.len();
            if scope[len - 1].id > i {
                break;
            }

            if !(len >= 2 && scope[len - 1].id == i && scope[len - 2].id == k) {
                continue;
            }

            let mut vals_needed = scope.iter().map(|var| vals[var.id].unwrap());
            if !eval(&mut vals_needed) {
                broken_constraint = Some(scope);
                break;
            }
        }

        broken_constraint
    }
}

// https://cs.uwaterloo.ca/~vanbeek/Publications/jair01.pdf
