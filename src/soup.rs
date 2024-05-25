use lambda_calculus::{app, abs, Var, Term};
use rand::{thread_rng, Rng};
use crate::config;

/// The principal AlChemy object. The `Soup` struct contains a set of
/// lambda expressions, and rules for composing and filtering them.
#[derive(Debug)]
pub struct Soup {
    expressions: Vec<Term>,
    reaction_rules: Vec<Term>,
    reduction_limit: usize,

    maintain_constant_population_size: bool,
    discard_copy_actions: bool,
    discard_identity: bool,
    discard_free_variable_expressions: bool,
    discard_parents: bool,
}

/// Stores the size and number of reductions for a collision
struct CollisionResult {
    pub size: u32,
    pub reductions: usize, 
}

/// The result of composing a vector `v` of 2-ary lambda expressions with
/// the expressions A and B.
struct ReactionResult {
    pub collision_results: Vec<CollisionResult>,

    /// Size of A
    pub left_size: u32,

    /// Size of B
    pub right_size: u32,
}

impl Soup {
    /// Generate an empty soup with the following configuration options:
    ///
    pub fn new() -> Self {
        Soup {
            expressions: Vec::new(),
            reaction_rules: vec![
                abs(abs(app(Var(1), Var(2)))), // \x. \y. x y
            ],
            reduction_limit: 100000,

            maintain_constant_population_size: true,
            discard_copy_actions: true,
            discard_identity: true,
            discard_free_variable_expressions: true,
            discard_parents: false,
        }
        
    }

    /// Generate an empty soup from a given `config` object.
    pub fn from_config(cfg: &config::Config) -> Self {
        Soup {
            expressions: Vec::new(),
            reaction_rules: cfg.rules.iter().map(|r| {
                lambda_calculus::parse(r, lambda_calculus::Classic).unwrap()
            }).collect(),
            reduction_limit: cfg.reduction_cutoff,
            
            maintain_constant_population_size: cfg.maintain_constant_population_size,
            discard_copy_actions: cfg.discard_copy_actions,
            discard_parents: cfg.discard_parents,
            discard_identity: cfg.discard_identity,
            discard_free_variable_expressions: cfg.discard_free_variable_expressions,
        }
    }



    /// Set the reduction limit of the soup
    pub fn set_limit(&mut self, limit: usize) {
        self.reduction_limit = limit;
    }

    /// Add a filter to the soup. If a filter is active, all expressions
    /// satisfying the conditions of the filter are removed from the soup.
    // pub fn add_filter(&mut self, filter: Filter) {
    //     self.filter.set(filter);
    // }

    /// Introduce all expressions in `expressions` into the soup, without
    /// reduction.
    pub fn perturb(&mut self, expressions: &mut Vec<Term>) {
        self.expressions.append(expressions);
    }

    /// Return the result of ((`rule` `left`) `right`), up to a limit of
    /// `self.reduction_limit`
    fn collide(&self, rule: Term, left: Term, right: Term) -> Option<(Term, usize)> {
        let mut expr = app!(rule, left.clone(), right.clone());
        let n = expr.reduce(lambda_calculus::HNO, self.reduction_limit);
        if n == self.reduction_limit {
            return None;
        } 

        let identity = abs(Var(1));
        if expr.is_isomorphic_to(&identity) && self.discard_identity {
            return None;
        }

        let is_copy_action = expr.is_isomorphic_to(&left) || expr.is_isomorphic_to(&right);
        if is_copy_action && self.discard_copy_actions {
            return None;
        }

        if expr.has_free_variables() && self.discard_free_variable_expressions {
            return None;
        }

        Some((expr, n))

    }

    // TODO: This is a huge monolith, decompose into something neater
    /// Produce one atomic reaction on the soup.
    fn react(&mut self) -> Option<ReactionResult> {
        let mut rng = thread_rng();
        let n_expr = self.expressions.len();

        // Remove two distinct expressions randomly from the soup
        let i = rng.gen_range(0..n_expr);
        let left = &self.expressions.swap_remove(i);
        let left_size = left.max_depth();

        let j = rng.gen_range(0..n_expr - 1);
        let right = &self.expressions.swap_remove(j);
        let right_size = right.max_depth();

        // Record collision information
        let mut buf = Vec::with_capacity(self.reaction_rules.len());
        let mut collision_results = Vec::with_capacity(self.reaction_rules.len());

        // Collide expressions
        for rule in &self.reaction_rules {
            let result = self.collide(rule.clone(), left.clone(), right.clone());
            if let Some((value, n)) = result {
                let datum = CollisionResult {
                    reductions: n,
                    size: value.max_depth()
                };
                collision_results.push(datum);
                buf.push(value);
            } else {
                return None;
            }
        }

        // Add collision results to soup
        self.expressions.append(&mut buf);

        // Add removed parents back into the soup, if necessary
        if !self.discard_parents {
            self.expressions.push(left.clone());
            self.expressions.push(right.clone());
        }

        // Remove additional expressions, if required.
        if self.maintain_constant_population_size {
            for _ in 0..(self.reaction_rules.len()) {
                let k = rng.gen_range(0..self.expressions.len());
                self.expressions.swap_remove(k);
            }
        }

        // Return collision log
        Some(ReactionResult {
            collision_results,
            left_size,
            right_size,
        })
    }

    /// Simulate the soup for `n` collisions.
    pub fn simulate_for(&mut self, n: usize) {
        for i in 0..n {
            // print!("reaction {:?}", i);
            println!(
                "reaction {:?} {}",
                i,
                if let Some(result) = self.react() {
                    format!("successful with {} reductions between expressions of sizes {} and {}, and produces an expression of size {}",
                            result.left_size, result.right_size, result.collision_results[0].reductions, result.collision_results[0].size)
                } else {
                    "failed".to_string()
                }
            )
        }
    }
}