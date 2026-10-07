#![allow(clippy::all)]
#![allow(warnings)]

use async_std::task::{block_on, spawn};
use futures::{stream::FuturesUnordered, StreamExt};
use lambda_calculus::Term;

use crate::{
    config::{self, ConfigSeed},
    generators::BTreeGen,
    lambda::recursive::LambdaSoup,
};

fn experiment_soup(seed: ConfigSeed) -> LambdaSoup {
    LambdaSoup::from_config(&config::Reactor {
        rules: vec![String::from("\\x.\\y.x y")],
        discard_copy_actions: false,
        discard_identity: false,
        discard_free_variable_expressions: true,
        maintain_constant_population_size: true,
        discard_parents: false,
        reduction_cutoff: 8000,
        size_cutoff: 1000,
        seed,
    })
}

fn experiment_gen(seed: ConfigSeed) -> BTreeGen {
    BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed,
    })
}

async fn simulate_soup(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
) -> (LambdaSoup, usize, f32) {
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
    soup.add_lambda_expressions(sample);
    let n_successes = soup.simulate_for(run_length, false);
    let failure_rate = 1f32 - n_successes as f32 / run_length as f32;
    (soup, id, failure_rate)
}

async fn simulate_soup_and_produce_entropies(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
    polling_interval: usize,
) -> (usize, Vec<f32>) {
    let mut seed: [u8; 32] = [0; 32];
    let bytes = id.to_le_bytes();
    seed[..bytes.len()].copy_from_slice(&bytes);
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
    soup.add_lambda_expressions(sample);
    let data = soup.simulate_and_poll(run_length, polling_interval, false, |s: &LambdaSoup| {
        s.population_entropy()
    });
    (id, data)
}

pub fn entropy_time_series() {
    let mut gen = experiment_gen(ConfigSeed::new([0; 32]));
    let mut futures = FuturesUnordered::new();
    let run_length = 10000000;
    let polling_interval = 1000;
    let polls = run_length / polling_interval;
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(simulate_soup_and_produce_entropies(
            sample.into_iter(),
            i,
            run_length,
            polling_interval,
        )));
    }

    print!("Soup, ");
    for i in 0..polls {
        print!("{}, ", i)
    }
    println!();
    while let Some((id, data)) = block_on(futures.next()) {
        print!("{}, ", id);
        for i in data {
            print!("{}, ", i)
        }
        println!();
    }
}

pub fn entropy_and_failures() {
    let mut gen = experiment_gen(ConfigSeed::new([0; 32]));
    let mut futures = FuturesUnordered::new();
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(simulate_soup(sample.into_iter(), i, 10000000)));
    }

    let mut data = Vec::new();
    println!("Soup, Entropy, Failure rate");
    while let Some((soup, id, failure_rate)) = block_on(futures.next()) {
        let entropy = soup.population_entropy();
        println!("{}, {}, {}", id, entropy, failure_rate);
        data.push(entropy);
    }
}

pub fn sync_entropy_and_failures() {
    let mut gen = experiment_gen(ConfigSeed::new([0; 32]));

    for i in 0..100 {
        let sample = gen.generate_n(1000);
        let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
        soup.add_lambda_expressions(sample);
        soup.simulate_for(100000, false);
        let entropy = soup.population_entropy();
        println!("{}: {}", i, entropy);
    }
}
