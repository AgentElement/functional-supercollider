use lambda_calculus::Term::{self, Abs};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::config;
use crate::config::GenConfig;

struct BTree {
    n: u32,
    left: Option<Box<BTree>>,
    right: Option<Box<BTree>>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Standardization {
    Prefix,
    Postfix,
    None,
}

impl BTree {
    fn new(n: u32) -> BTree {
        BTree {
            n,
            left: None,
            right: None,
        }
    }

    fn insert(&mut self, n: u32) {
        let child = BTree::new(n);
        match (&mut self.left, &mut self.right, n <= self.n) {
            (None, _, true) => self.left = Some(Box::new(child)),
            (_, None, false) => self.right = Some(Box::new(child)),
            (Some(t), _, true) | (_, Some(t), false) => t.insert(n),
        };
    }

    fn to_lambda_h(
        &self,
        rng: &mut ChaCha8Rng,
        freevar_p: f64,
        max_free_vars: u32,
        depth: u32,
    ) -> Term {
        match (&self.left, &self.right) {
            (None, None) => {
                let var = if rng.gen_bool(freevar_p) || depth == 0 {
                    depth + rng.gen_range(1..=max_free_vars)
                } else {
                    rng.gen_range(1..=depth)
                };
                Term::Var(var as usize)
            }
            (Some(t), None) | (None, Some(t)) => Term::Abs(Box::new(t.to_lambda_h(
                rng,
                freevar_p,
                max_free_vars,
                depth + 1,
            ))),
            (Some(l), Some(r)) => {
                let left = l.to_lambda_h(rng, freevar_p, max_free_vars, depth);
                let right = r.to_lambda_h(rng, freevar_p, max_free_vars, depth);
                Term::App(Box::new((left, right)))
            }
        }
    }

    fn to_lambda(&self, rng: &mut ChaCha8Rng, freevar_p: f64, max_free_vars: u32) -> Term {
        self.to_lambda_h(rng, freevar_p, max_free_vars, 0)
    }
}

pub struct BTreeGen {
    n: u32,
    freevar_p: f64,
    max_free_vars: u32,
    std: Standardization,

    seed: [u8; 32],
    rng: ChaCha8Rng,
}

impl Default for BTreeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl BTreeGen {
    pub fn new() -> BTreeGen {
        BTreeGen::from_config(&config::BTreeGen::new())
    }

    pub fn from_config(cfg: &config::BTreeGen) -> BTreeGen {
        let seed = cfg.seed.get();
        let rng = ChaCha8Rng::from_seed(seed);
        BTreeGen {
            n: cfg.size,
            freevar_p: cfg.freevar_generation_probability,
            max_free_vars: cfg.n_max_free_vars,
            std: cfg.standardization,

            seed,
            rng,
        }
    }

    pub fn generate(&mut self) -> Term {
        let n = self.n;
        assert!(
            n > 0,
            "btree generator does not produce zero-sized expressions."
        );
        let mut permutation = (0..n).collect::<Vec<u32>>();
        permutation.shuffle(&mut self.rng);
        let mut tree = BTree::new(permutation[0]);
        permutation.iter().skip(1).for_each(|i| tree.insert(*i));
        let lambda = tree.to_lambda(&mut self.rng, self.freevar_p, self.max_free_vars);
        match self.std {
            Standardization::Postfix => BTreeGen::postfix_standardize(lambda),
            Standardization::Prefix => BTreeGen::prefix_standardize(lambda),
            Standardization::None => lambda,
        }
    }

    pub fn generate_n(&mut self, n: usize) -> Vec<Term> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.generate())
        }
        v
    }

    pub fn generate_n_unique(&mut self, n: usize) -> Vec<Term> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut results: Vec<Term> = Vec::with_capacity(n);

        let max_attempts = n * 20;
        let mut attempts = 0;

        while results.len() < n && attempts < max_attempts {
            let term = self.generate();
            let key = term.to_string();
            if seen.insert(key) {
                results.push(term);
            }
            attempts += 1;
        }

        results
    }

    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    fn postfix_standardize(mut t: Term) -> Term {
        let mut depth = 0;
        while t.has_free_variables() {
            t = Abs(Box::new(t));
            depth += 1;
        }
        for _ in 0..depth {
            // Identity function: \x. 1 (in De Bruijn, \x. x is Abs(Var(1)))
            let identity = Abs(Box::new(Term::Var(1)));
            t = Term::App(Box::new((t, identity)));
        }
        t
    }

    /// Add abstractions until the expression has no free variables
    fn prefix_standardize(mut t: Term) -> Term {
        // This is horrible, and can easily be made more efficient. Fortunaltely,
        // lambda-expression generation is a one-off thing!
        while t.has_free_variables() {
            t = Abs(Box::new(t))
        }
        t
    }
}

pub struct FontanaGen {
    min_depth: u32,
    max_depth: u32,
    abs_prob: (f32, f32),
    app_prob: (f32, f32),
    abs_incr: f32,
    app_incr: f32,
    seed: [u8; 32],
    rng: ChaCha8Rng,
}

impl FontanaGen {
    pub fn new(
        min_depth: u32,
        mut max_depth: u32,
        mut abs_prob: (f32, f32),
        mut app_prob: (f32, f32),
        seed: [u8; 32],
    ) -> FontanaGen {
        max_depth = max_depth.max(1);
        abs_prob.0 = abs_prob.0.clamp(0.0, 1.0);
        abs_prob.1 = abs_prob.1.clamp(0.0, 1.0);
        app_prob.0 = app_prob.0.clamp(0.0, 1.0);
        app_prob.1 = app_prob.1.clamp(0.0, 1.0);

        let steps = (max_depth - 1).max(1);
        let abs_incr = (abs_prob.1 - abs_prob.0) / (steps as f32);
        let app_incr = (app_prob.1 - app_prob.0) / (steps as f32);

        FontanaGen {
            min_depth: min_depth.min(max_depth.saturating_sub(1)),
            max_depth,
            abs_prob,
            app_prob,
            abs_incr,
            app_incr,
            seed,
            rng: ChaCha8Rng::from_seed(seed),
        }
    }

    pub fn from_config(cfg: &config::FontanaGen) -> FontanaGen {
        let seed = cfg.seed.get();

        FontanaGen::new(
            cfg.min_depth,
            cfg.max_depth,
            (
                cfg.abstraction_prob_range.0 as f32,
                cfg.abstraction_prob_range.1 as f32,
            ),
            (
                cfg.application_prob_range.0 as f32,
                cfg.application_prob_range.1 as f32,
            ),
            seed,
        )
    }

    pub fn generate(&mut self) -> Term {
        self.rand_lambda(0, 0, self.abs_prob.0, self.app_prob.0)
    }

    pub fn generate_n(&mut self, n: usize) -> Vec<Term> {
        (0..n).map(|_| self.generate()).collect()
    }

    pub fn generate_n_unique(&mut self, n: usize) -> Vec<Term> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut results: Vec<Term> = Vec::with_capacity(n);

        let max_attempts = n * 20;
        let mut attempts = 0;

        while results.len() < n && attempts < max_attempts {
            let term = self.generate();
            let key = term.to_string();
            if seen.insert(key) {
                results.push(term);
            }
            attempts += 1;
        }

        results
    }

    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    pub fn rand_lambda(&mut self, depth: u32, abs_depth: u32, p_abs: f32, p_app: f32) -> Term {
        // Terminal case: max depth reached
        if depth >= self.max_depth {
            if abs_depth > 0 {
                return self.sample_variable(abs_depth);
            }
            return Term::Abs(Box::new(Term::Var(1)));
        }

        let next_abs = p_abs + self.abs_incr;
        let next_app = p_app + self.app_incr;

        let (p_abs_eff, p_app_eff) = Self::clamp_probabilities(p_abs, p_app);
        let coin: f32 = self.rng.gen();

        if coin <= p_abs_eff {
            return Term::Abs(Box::new(self.rand_lambda(
                depth + 1,
                abs_depth + 1,
                next_abs,
                next_app,
            )));
        }

        // Below min_depth or no binders: can't emit Var, must pick Abs or App
        if abs_depth == 0 || depth < self.min_depth {
            return Term::App(Box::new((
                self.rand_lambda(depth + 1, abs_depth, next_abs, next_app),
                self.rand_lambda(depth + 1, abs_depth, next_abs, next_app),
            )));
        }

        if coin <= p_abs_eff + p_app_eff {
            return Term::App(Box::new((
                self.rand_lambda(depth + 1, abs_depth, next_abs, next_app),
                self.rand_lambda(depth + 1, abs_depth, next_abs, next_app),
            )));
        }

        self.sample_variable(abs_depth)
    }

    fn sample_variable(&mut self, abs_depth: u32) -> Term {
        debug_assert!(
            abs_depth > 0,
            "sample_variable called with no enclosing abstractions"
        );
        Term::Var(self.rng.gen_range(1..=abs_depth) as usize)
    }

    fn clamp_probabilities(p_abs: f32, p_app: f32) -> (f32, f32) {
        let abs = p_abs.clamp(0.0, 1.0);
        let remaining = 1.0 - abs;
        let app = p_app.clamp(0.0, remaining);
        (abs, app)
    }
}
