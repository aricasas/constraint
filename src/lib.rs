use hashbrown::HashMap;
use std::fmt::Debug;

type Universe = i32;
type Evaluation = Box<dyn Fn(&Vec<Universe>) -> bool>;
type Candidate = Vec<Universe>;

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
                    constraints: constraints.into_iter().collect(),
                },
            )
    }

    fn make_node_consistency(mut self) -> Self {
        for i in 0..self.variables.len() {
            let var = self.variables[i];
            let domain = &mut self.domains[i].values;

            if let Some(eval) = self.constraints.remove(&vec![var]) {
                domain.retain(|&vx| eval(&vec![vx]));
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
                    .is_some_and(|eval| eval(&vec![vx, vy]))
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
}

pub struct PropagatedProblem {
    pub variables: Vec<Variable>,
    pub domains: Vec<Domain>,
    pub constraints: Vec<(Vec<Variable>, Evaluation)>,
}

impl PropagatedProblem {
    pub fn solve_backtracking(&self) -> Option<Candidate> {
        self.backtrack(&Vec::new())
    }
    fn backtrack(&self, candidate: &Candidate) -> Option<Candidate> {
        if self.reject(candidate) {
            return None;
        }
        if self.accept(candidate) {
            return Some(candidate.clone());
        }
        let mut s = self.first(candidate);
        while let Some(ss) = s {
            let solution = self.backtrack(&ss);
            if solution.is_some() {
                return solution;
            }

            s = self.next(&ss);
        }

        None
    }
    /// Returns true if candidate values are inconsistent with constraints
    fn reject(&self, candidate: &Candidate) -> bool {
        let k = candidate.len();
        if k == 0 {
            return false;
        }

        let curr_var = self.variables[k - 1];

        let to_check = self
            .constraints
            .iter()
            .filter(|constraint| constraint.0.last() == Some(&curr_var));

        for constraint in to_check {
            let vals_needed = constraint.0.iter().map(|var| candidate[var.id]).collect();
            if !constraint.1(&vals_needed) {
                return true;
            }
        }

        false
    }
    /// Returns true if candidate values are consistent and complete with constraints
    fn accept(&self, candidate: &Candidate) -> bool {
        candidate.len() == self.variables.len()
    }
    fn first(&self, candidate: &Candidate) -> Option<Candidate> {
        let k = candidate.len();
        if k == self.variables.len() {
            None
        } else {
            let first_val_next_var = self.domains[k].values[0];
            let mut next_cand = candidate.clone();
            next_cand.push(first_val_next_var);
            Some(next_cand)
        }
    }
    fn next(&self, candidate: &Candidate) -> Option<Candidate> {
        let k = candidate.len();
        if candidate.last() == self.domains[k - 1].values.last() {
            return None;
        }

        if let Some(curr_val) = candidate.last() {
            let mut new_cand = candidate.clone();
            if let Ok(i) = self.domains[k - 1].values.binary_search(curr_val) {
                new_cand[k - 1] = self.domains[k - 1].values[i + 1];
                Some(new_cand)
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub mod sudoku;
