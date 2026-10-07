use core::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use crate::config;
use crate::supercollider::{Collider, Particle, Residue, Soup};
use lambda_calculus::Term::Var;
use lambda_calculus::{abs, app, Term};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub type LambdaSoup =
    Soup<LambdaParticle, AlchemyCollider, LambdaCollisionOk, LambdaCollisionError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaParticle {
    pub expr: Term,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyCollider {
    rlimit: usize,
    slimit: usize,
    disallow_recursive: bool,
    reaction_rules: Vec<Term>,
    discard_copy_actions: bool,
    discard_identity: bool,
    discard_free_variable_expressions: bool,
}

/// The result of composing a vector `v` of 2-ary lambda expressions with
/// the expressions A and B.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaCollisionOk {
    pub results: Vec<LambdaParticle>,
    pub reductions: Vec<usize>,
    pub sizes: Vec<usize>,

    /// Size of A
    pub left_size: usize,

    /// Size of B
    pub right_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LambdaCollisionError {
    ExceedsReductionLimit,
    NotEnoughExpressions,
    IsIdentity,
    IsParent,
    HasFreeVariables,
    ExceedsDepthLimit,
    BadArgument,
}

impl LambdaParticle {
    pub fn get_underlying_term(&self) -> &Term {
        &self.expr
    }
}

pub fn has_two_args(expr: &Term) -> bool {
    if let Term::Abs(ref body) = expr {
        if let Term::Abs(_) = **body {
            return true;
        }
    }
    false
}

// Check if expr has the form \x1. ... \xn. var for n >= 2
pub fn is_truthy(expr: &Term) -> bool {
    if let Term::Abs(ref body) = expr {
        // Hopefully if let chaining becomes stable someday
        if let Term::Abs(ref var) = **body {
            if let Term::Var(_) = **var {
                return true;
            }
        }
        return is_truthy(body);
    }
    false
}

pub fn reduce_with_limit(
    expr: &mut Term,
    rlimit: usize,
    slimit: usize,
) -> Result<usize, LambdaCollisionError> {
    let mut n = 0;
    for _ in 0..rlimit {
        if expr.reduce(lambda_calculus::HAP, 1) == 0 {
            break;
        }

        // WARNING: This is EXTREMELY expensive. Calling max_depth is log(depth), and is done
        // per reduction step. Remove when possible.
        let depth = expr.size();
        if depth > slimit {
            return Err(LambdaCollisionError::ExceedsDepthLimit);
        }
        n += 1;
    }
    Ok(n)
}

impl AlchemyCollider {
    pub fn from_config(cfg: &config::Reactor) -> Self {
        Self {
            rlimit: cfg.reduction_cutoff,
            slimit: cfg.size_cutoff,
            disallow_recursive: false,
            reaction_rules: cfg
                .rules
                .iter()
                .map(|r| lambda_calculus::parse(r, lambda_calculus::Classic).unwrap())
                .collect(),
            discard_copy_actions: cfg.discard_copy_actions,
            discard_identity: cfg.discard_identity,
            discard_free_variable_expressions: cfg.discard_free_variable_expressions,
        }
    }

    fn collide(
        &self,
        left: LambdaParticle,
        right: LambdaParticle,
    ) -> Result<LambdaCollisionOk, LambdaCollisionError> {
        let lt = left.expr;
        let rt = right.expr;
        let mut collision_results = Vec::with_capacity(self.reaction_rules.len());

        for rule in &self.reaction_rules {
            let mut expr = app!(rule.clone(), lt.clone(), rt.clone());
            let n = reduce_with_limit(&mut expr, self.rlimit, self.slimit)?;
            let size = expr.size();

            if n == self.rlimit {
                return Err(LambdaCollisionError::ExceedsReductionLimit);
            }

            let identity = abs(Var(1));
            if expr.is_isomorphic_to(&identity) && self.discard_identity {
                return Err(LambdaCollisionError::IsIdentity);
            }

            let is_copy_action = expr.is_isomorphic_to(&lt) || expr.is_isomorphic_to(&rt);
            if is_copy_action && self.discard_copy_actions {
                return Err(LambdaCollisionError::IsParent);
            }

            if expr.has_free_variables() && self.discard_free_variable_expressions {
                return Err(LambdaCollisionError::HasFreeVariables);
            }

            let expr = LambdaParticle { expr };

            collision_results.push((expr, n, size))
        }
        Ok(LambdaCollisionOk {
            results: collision_results.iter().map(|t| t.0.clone()).collect(),
            reductions: collision_results.iter().map(|t| t.1).collect(),
            sizes: collision_results.iter().map(|t| t.2).collect(),
            left_size: lt.size(),
            right_size: rt.size(),
        })
    }
}

impl Particle for LambdaParticle {
    fn compose(&self, other: &Self) -> Self {
        LambdaParticle {
            expr: lambda_calculus::app!(self.expr.clone(), other.expr.clone()),
        }
    }

    fn is_isomorphic_to(&self, other: &Self) -> bool {
        self.expr.is_isomorphic_to(&other.expr)
    }
}

impl Collider<LambdaParticle, LambdaCollisionOk, LambdaCollisionError> for AlchemyCollider {
    /// Return the result of ((`rule` `left`) `right`), up to a limit of
    /// `self.reduction_limit`.
    fn collide(
        &self,
        left: LambdaParticle,
        right: LambdaParticle,
    ) -> Result<LambdaCollisionOk, LambdaCollisionError> {
        self.collide(left, right)
    }
}

impl Residue<LambdaParticle> for LambdaCollisionOk {
    fn particles(&self) -> impl Iterator<Item = LambdaParticle> {
        self.results.iter().cloned()
    }

    fn count(&self) -> usize {
        self.results.len()
    }
}

impl fmt::Display for LambdaCollisionOk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt("no message", f)
    }
}

impl fmt::Display for LambdaCollisionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LambdaCollisionError::IsIdentity => {
                Display::fmt("collision result is identity function", f)
            }
            LambdaCollisionError::IsParent => {
                Display::fmt("collision result is isomorphic to parent", f)
            }
            LambdaCollisionError::ExceedsReductionLimit => {
                Display::fmt("collision exceeds reduction limit", f)
            }
            LambdaCollisionError::NotEnoughExpressions => {
                Display::fmt("not enough expressions for further reactions", f)
            }
            LambdaCollisionError::HasFreeVariables => {
                Display::fmt("collision result has free variables", f)
            }

            LambdaCollisionError::ExceedsDepthLimit => {
                Display::fmt("expression exceeds depth limit during reduction", f)
            }
            LambdaCollisionError::BadArgument => Display::fmt(
                "argument is truth-like or doesn't use all of own arguments",
                f,
            ),
        }
    }
}

impl std::error::Error for LambdaCollisionError {}

impl fmt::Display for LambdaParticle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&format!("{:?}", self.expr), f)
    }
}

impl Default for LambdaSoup {
    fn default() -> Self {
        Self::new()
    }
}

impl LambdaSoup {
    /// Generate an empty soup with the following configuration options:
    pub fn new() -> Self {
        LambdaSoup::from_config(&config::Reactor::new())
    }

    /// Generate an empty soup from a given `config` object.
    pub fn from_config(cfg: &config::Reactor) -> Self {
        let seed = cfg.seed.get();
        let rng = ChaCha8Rng::from_seed(seed);
        Self {
            expressions: Vec::new(),
            collider: AlchemyCollider::from_config(cfg),
            maintain_constant_population_size: cfg.maintain_constant_population_size,
            discard_parents: cfg.discard_parents,
            rng,
            n_collisions: 0,
            t: PhantomData,
            e: PhantomData,
        }
    }

    pub fn add_lambda_expressions(&mut self, expressions: impl IntoIterator<Item = Term>) {
        self.expressions
            .extend(expressions.into_iter().map(|t| LambdaParticle { expr: t }))
    }

    pub fn perturb_lambda_expressions<I>(&mut self, nterms: usize, expressions: I)
    where
        I: IntoIterator<Item = Term>,
        <I as IntoIterator>::IntoIter: Clone,
    {
        if self.maintain_constant_population_size {
            for _ in 0..nterms {
                let k = self.rng.gen_range(0..self.expressions.len());
                self.expressions.swap_remove(k);
            }
        }
        self.add_lambda_expressions(expressions.into_iter().cycle().take(nterms))
    }

    pub fn lambda_expressions(&self) -> impl Iterator<Item = &Term> {
        self.expressions.iter().map(|e| e.get_underlying_term())
    }

    pub fn population_of(&self, item: &Term) -> usize {
        self.lambda_expressions()
            .filter(|p| p.is_isomorphic_to(item))
            .count()
    }
}
