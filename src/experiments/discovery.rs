#![allow(clippy::all)]
#![allow(warnings)]

use async_std::task::{block_on, spawn};
use futures::stream::{FuturesUnordered, StreamExt};
use lambda_calculus::{
    data::num::church::{add, succ},
    Term,
};
use rand::random;

use crate::{
    config::{self, ConfigSeed},
    generators::BTreeGen,
    lambda::recursive::reduce_with_limit,
    utils::dump_series_to_file,
};

use super::{
    kinetics::{general_run, general_test_run, RunParams},
    magic_test_function::{
        addtwo, coadd, ski_sample, symmetric_skip_sample, test_add, test_addtwo, test_succ,
    },
};

fn experiment_gen(seed: ConfigSeed) -> BTreeGen {
    BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed,
    })
}

pub fn measure_initial_population() {
    for (i, term) in [succ(), add()].iter().enumerate() {
        let series = (0..1000)
            .map(|_| {
                let random_seed = ConfigSeed::new(random::<[u8; 32]>());
                let mut gen = experiment_gen(random_seed);
                gen.generate_n(10000)
                    .iter_mut()
                    .map(|mut t| {
                        let r = reduce_with_limit(&mut t, 1000, 8000);
                        (r, t)
                    })
                    .filter(|(r, t)| r.is_ok() && t.is_isomorphic_to(&term))
                    .count()
            })
            .collect::<Vec<_>>();
        dump_series_to_file("initial_population_counts", &series, &[i])
            .expect("Cannot write to file");
    }
}

fn parallel_run_executor<F>(fname: &str, isomorphics: &[Term], sample_generator: F)
where
    F: Fn() -> Vec<Term>,
{
    let mut futures = FuturesUnordered::new();
    let sample_size = 5000;
    for i in 0..1000 {
        let random_seed = ConfigSeed::new(random::<[u8; 32]>());
        let samples = sample_generator();

        let params = RunParams {
            id: vec![i],
            seed: random_seed,
            count_each_poll: isomorphics.to_vec(),
            perturbation_interval: 10,
            polling_interval: 1000,
            run_length: 100000,
            perturbation_size: 0,
        };

        let run = general_run(vec![], samples, 0, sample_size, params);
        futures.push(spawn(run));
    }
    while let Some((id, series)) = block_on(futures.next()) {
        dump_series_to_file(fname, &series, &id).expect("Cannot write to file");
    }
}

fn parallel_test_run_executor<F, T>(
    fname: &str,
    isomorphics: &[Term],
    sample_generator: F,
    test_generator: Vec<T>,
) where
    F: Fn() -> Vec<Term>,
    T: Fn() -> Term + Send + Clone + 'static,
{
    let mut futures = FuturesUnordered::new();
    let sample_size = 4000;
    let test_size = 1000;
    for i in 0..1000 {
        let random_seed = ConfigSeed::new(random::<[u8; 32]>());
        let samples = sample_generator();

        let params = RunParams {
            id: vec![i],
            seed: random_seed,
            count_each_poll: isomorphics.to_vec(),
            perturbation_interval: 10,
            polling_interval: 1000,
            run_length: 100000,
            perturbation_size: 0,
        };

        let run = general_test_run(
            vec![],
            samples,
            test_generator.clone(),
            0,
            sample_size,
            test_size,
            params,
        );
        futures.push(spawn(run));
    }
    while let Some((id, series)) = block_on(futures.next()) {
        dump_series_to_file(fname, &series, &id).expect("Cannot write to file");
    }
}

pub fn add_scc_population_from_random_inputs() {
    parallel_run_executor(
        "add_scc_population_from_random_inputs",
        &[succ(), add()],
        || {
            let random_seed = ConfigSeed::new(random::<[u8; 32]>());
            experiment_gen(random_seed).generate_n(5000)
        },
    )
}

pub fn add_scc_population_from_ski_inputs() {
    parallel_run_executor(
        "add_scc_population_from_ski_inputs",
        &[succ(), add()],
        || ski_sample(),
    )
}

pub fn add_scc_population_from_skip_inputs() {
    parallel_run_executor(
        "add_scc_population_from_skip_inputs",
        &[succ(), add()],
        || symmetric_skip_sample(),
    )
}

pub fn scc_population_from_random_inputs_with_tests() {
    let tests = vec![|| test_succ(random::<usize>() % 20)];
    parallel_test_run_executor(
        "scc_population_from_random_inputs_with_tests",
        &[succ(), add()],
        || {
            let random_seed = ConfigSeed::new(random::<[u8; 32]>());
            experiment_gen(random_seed).generate_n(5000)
        },
        tests,
    )
}

pub fn add_population_from_random_inputs_with_tests() {
    let tests = vec![|| test_add(random::<usize>() % 20, random::<usize>() % 20)];
    parallel_test_run_executor(
        "add_population_from_random_inputs_with_tests",
        &[succ(), add()],
        || {
            let random_seed = ConfigSeed::new(random::<[u8; 32]>());
            experiment_gen(random_seed).generate_n(5000)
        },
        tests,
    )
}

pub fn add_population_from_random_inputs_with_add_succ_tests() {
    let tests = vec![
        || test_add(random::<usize>() % 20, random::<usize>() % 20),
        || test_succ(random::<usize>() % 20),
    ];
    parallel_test_run_executor(
        "add_population_from_random_inputs_with_add_succ_tests",
        &[succ(), add()],
        || {
            let random_seed = ConfigSeed::new(random::<[u8; 32]>());
            experiment_gen(random_seed).generate_n(5000)
        },
        tests,
    )
}

// Successor sawtooth figure
pub fn scc_population_from_ski_inputs_with_tests() {
    let tests = vec![|| test_succ(random::<usize>() % 20)];
    parallel_test_run_executor(
        "scc_population_from_ski_inputs_with_tests",
        &[succ(), add()],
        || ski_sample(),
        tests,
    )
}

pub fn add_population_from_ski_inputs_with_tests() {
    let tests = vec![|| test_add(random::<usize>() % 20, random::<usize>() % 20)];
    parallel_test_run_executor(
        "add_random_pop_series_test",
        &[succ(), add()],
        || ski_sample(),
        tests,
    )
}

// Add sawtooth figure (ski, atomic)
pub fn add_population_from_ski_inputs_with_add_succ_tests() {
    let tests = vec![
        || test_add(random::<usize>() % 20, random::<usize>() % 20),
        || test_succ(random::<usize>() % 20),
    ];
    parallel_test_run_executor(
        "add_ski_addsucc_tests",
        &[succ(), add(), coadd()],
        || ski_sample(),
        tests,
    )
}

// Add sawtooth figure (ski, batched)
pub fn add_population_from_ski_inputs_with_batchedadd_succ_tests() {
    let tests = vec![
        || test_add(random::<usize>() % 20, random::<usize>() % 20),
        || test_succ(random::<usize>() % 20),
    ];
    parallel_test_run_executor(
        "add_ski_batchedaddsucc_tests",
        &[succ(), add(), coadd()],
        || ski_sample(),
        tests,
    )
}

// Add sawtooth figure (skip, atomic)
pub fn add_population_from_skip_inputs_with_add_succ_tests() {
    let tests = vec![
        || test_add(random::<usize>() % 20, random::<usize>() % 20),
        || test_succ(random::<usize>() % 20),
    ];
    parallel_test_run_executor(
        "add_skip_addsucc_tests",
        &[succ(), add(), coadd()],
        || symmetric_skip_sample(),
        tests,
    )
}

// Addtwo sawtooth figure
pub fn addtwo_population_from_ski_inputs_with_addtwo_tests() {
    let tests = vec![|| test_addtwo(random::<usize>() % 20)];
    parallel_test_run_executor(
        "addtwo_ski_addtwo_tests",
        &[succ(), addtwo()],
        || ski_sample(),
        tests,
    )
}
