use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use rand::Rng;
use rand_chacha::ChaCha8Rng;

pub trait Particle {
    fn compose(&self, other: &Self) -> Self;

    fn is_isomorphic_to(&self, other: &Self) -> bool;
}

pub trait Collider<P, T, E>
where
    P: Particle,
{
    fn collide(&self, left: P, right: P) -> Result<T, E>;
}

pub trait Residue<P>
where
    P: Particle,
{
    fn particles(&self) -> impl Iterator<Item = P>;
    fn count(&self) -> usize;
}

/// A single logged reaction event, capturing parents, products, and outcome.
#[derive(Debug, Clone)]
pub struct ReactionRecord<P: Clone> {
    pub step: usize,
    pub left: P,
    pub right: P,
    pub products: Vec<P>,
    pub success: bool,
    pub error: Option<String>,
}

/// The principal AlChemy object. The `Soup` struct contains a set of
/// lambda expressions, and rules for composing and filtering them.
#[derive(Debug, Clone)]
pub struct Soup<P, C, T, E> {
    // All of these pub(crate)s here are hacky
    pub(crate) expressions: Vec<P>,
    pub(crate) n_collisions: usize,
    pub(crate) collider: C,

    pub(crate) maintain_constant_population_size: bool,
    pub(crate) discard_parents: bool,

    pub(crate) rng: ChaCha8Rng,

    // TODO: Figure out how to get rid of these horrible phantomdatas
    pub(crate) t: PhantomData<T>,
    pub(crate) e: PhantomData<E>,
}

pub struct Tape<P, C, T, E> {
    soup: Soup<P, C, T, E>,
    history: Vec<Soup<P, C, T, E>>,
    polling_interval: usize,
}

impl<P, C, T, E> Soup<P, C, T, E>
where
    P: Particle + Display + Clone,
    C: Collider<P, T, E> + Clone,
    T: Display + Clone + Residue<P>,
    E: Display + Clone + std::error::Error,
{
    /// Introduce all expressions in `expressions` into the soup, without
    /// reduction.
    pub fn perturb(&mut self, expressions: impl IntoIterator<Item = P>) {
        self.expressions.extend(expressions)
    }

    /// Produce one atomic reaction on the soup.
    pub fn react(&mut self) -> Result<T, E> {
        let n_expr = self.expressions.len();

        // Remove two distinct expressions randomly from the soup
        let i = self.rng.gen_range(0..n_expr);
        let left = self.expressions.swap_remove(i);

        let j = self.rng.gen_range(0..n_expr - 1);
        let right = self.expressions.swap_remove(j);

        // Add collision results to soup
        let result = self.collider.collide(left.clone(), right.clone());

        if let Ok(ref t) = result {
            self.perturb(t.particles());

            // Remove additional expressions, if required.
            if self.maintain_constant_population_size {
                for _ in 0..t.count() {
                    let k = self.rng.gen_range(0..self.expressions.len());
                    self.expressions.swap_remove(k);
                }
            }
        }

        // Add removed parents back into the soup, if necessary
        if !self.discard_parents {
            self.expressions.push(left);
            self.expressions.push(right);
        }

        result.clone()
    }

    /// Produce one atomic reaction and return a full record of who
    /// reacted with whom to produce what.
    pub fn react_logged(&mut self, step: usize) -> ReactionRecord<P> {
        let n_expr = self.expressions.len();

        let i = self.rng.gen_range(0..n_expr);
        let left = self.expressions.swap_remove(i);

        let j = self.rng.gen_range(0..n_expr - 1);
        let right = self.expressions.swap_remove(j);

        let result = self.collider.collide(left.clone(), right.clone());

        let record = match &result {
            Ok(ref t) => {
                let products: Vec<P> = t.particles().collect();
                self.perturb(products.iter().cloned());

                if self.maintain_constant_population_size {
                    for _ in 0..t.count() {
                        let k = self.rng.gen_range(0..self.expressions.len());
                        self.expressions.swap_remove(k);
                    }
                }

                ReactionRecord {
                    step,
                    left: left.clone(),
                    right: right.clone(),
                    products,
                    success: true,
                    error: None,
                }
            }
            Err(ref e) => ReactionRecord {
                step,
                left: left.clone(),
                right: right.clone(),
                products: vec![],
                success: false,
                error: Some(format!("{}", e)),
            },
        };

        if !self.discard_parents {
            self.expressions.push(left);
            self.expressions.push(right);
        }

        record
    }

    /// Simulate for `n` collisions, returning a full reaction log.
    /// Note: this allocates a Vec of n records. For very large n,
    /// consider using `simulate_for_logged_filtered` instead.
    pub fn simulate_for_logged(&mut self, n: usize) -> Vec<ReactionRecord<P>> {
        let mut log = Vec::with_capacity(n);
        for step in 0..n {
            log.push(self.react_logged(step));
        }
        log
    }

    /// Simulate for `n` collisions, returning only successful reaction records.
    pub fn simulate_for_logged_filtered(&mut self, n: usize) -> Vec<ReactionRecord<P>> {
        let mut log = Vec::new();
        for step in 0..n {
            let record = self.react_logged(step);
            if record.success {
                log.push(record);
            }
        }
        log
    }

    fn log_message_from_reaction(reaction: &Result<T, E>) -> String {
        match reaction {
            Ok(result) => format!("successful with {}", result),
            Err(message) => format!("failed because {}", message),
        }
    }

    /// Simulate the soup for `n` collisions. If `log` is set, then print
    /// out a log message for each reaction. Returns the number of successful reactions
    /// (the fraction of failed reactions).
    pub fn simulate_for(&mut self, n: usize, log: bool) -> usize {
        let mut n_successes = 0;
        for i in 0..n {
            let reaction = self.react();
            if reaction.is_ok() {
                n_successes += 1;
            }

            if log {
                let message = Self::log_message_from_reaction(&reaction);
                println!("reaction {:?} {}", i, message)
            }
        }
        n_successes
    }

    pub fn simulate_and_poll<F, R>(
        &mut self,
        n: usize,
        polling_interval: usize,
        log: bool,
        poller: F,
    ) -> Vec<R>
    where
        F: Fn(&Self) -> R,
    {
        let mut data: Vec<R> = Vec::new();
        for i in 0..n {
            let reaction = self.react();
            if (i % polling_interval) == 0 {
                data.push(poller(self))
            }
            if log {
                let message = Self::log_message_from_reaction(&reaction);
                println!("reaction {:?} {}", i, message)
            }
        }
        data
    }

    pub fn simulate_and_poll_with_killer<F, R>(
        &mut self,
        n: usize,
        polling_interval: usize,
        log: bool,
        killpoller: F,
    ) -> Vec<R>
    where
        F: Fn(&Self) -> (R, bool),
    {
        let mut data: Vec<R> = Vec::new();
        for i in 0..n {
            let reaction = self.react();
            if (i % polling_interval) == 0 {
                let (datum, should_kill) = killpoller(self);
                data.push(datum);
                if should_kill {
                    return data;
                };
            }
            if log {
                let message = Self::log_message_from_reaction(&reaction);
                println!("reaction {:?} {}", i, message)
            }
        }
        data
    }

    /// Simulate the soup for `n` collisions, recording the state of the soup every
    /// `polling_interval` reactions. If `log` is set, then print out a log message for each
    /// reaction
    pub fn simulate_and_record(
        &mut self,
        n: usize,
        polling_interval: usize,
        log: bool,
    ) -> Tape<P, C, T, E> {
        let mut history: Vec<Self> = Vec::new();
        for i in 0..n {
            let reaction = self.react();
            if (i % polling_interval) == 0 {
                history.push(self.clone())
            }
            if log {
                let message = Self::log_message_from_reaction(&reaction);
                println!("reaction {:?} {}", i, message)
            }
        }

        Tape::<P, C, T, E> {
            soup: self.clone(),
            history,
            polling_interval,
        }
    }

    /// Print out all expressions within the soup. Defaults to Church notation.
    pub fn print(&self) {
        for expression in &self.expressions {
            println!("{}", expression)
        }
    }

    /// Get an iterator over all expressions.
    pub fn expressions(&self) -> impl Iterator<Item = &P> {
        self.expressions.iter()
    }

    /// Get the number of expressions in the soup.
    pub fn len(&self) -> usize {
        self.expressions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }

    /// Get the number of successful collisions
    pub fn collisions(&self) -> usize {
        self.n_collisions
    }
}

impl<P, C, T, E> Tape<P, C, T, E>
where
    P: Particle + Display + Clone,
    C: Collider<P, T, E> + Clone,
    T: Display + Clone + Residue<P>,
    E: Display + Clone + std::error::Error,
{
    pub fn final_state(&self) -> &Soup<P, C, T, E> {
        &self.soup
    }

    pub fn history(&self) -> impl Iterator<Item = &Soup<P, C, T, E>> {
        self.history.iter()
    }

    pub fn polling_interval(&self) -> usize {
        self.polling_interval
    }
}
