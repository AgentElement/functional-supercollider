#![allow(clippy::all)]
#![allow(warnings)]

use async_std::task::{block_on, spawn};
use futures::{stream::FuturesUnordered, StreamExt};
use lambda_calculus::{app, Term};

use crate::{
    config::{self, ConfigSeed},
    generators::BTreeGen,
    lambda::recursive::{reduce_with_limit, LambdaSoup},
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

fn xorset_test(a: &Term, b: &Term) -> bool {
    if a.is_isomorphic_to(b) {
        return false;
    }

    let mut aa = app(a.clone(), a.clone());
    let mut ab = app(a.clone(), b.clone());
    let mut ba = app(b.clone(), a.clone());
    let mut bb = app(b.clone(), b.clone());

    let _ = reduce_with_limit(&mut aa, 512, 1024);
    let _ = reduce_with_limit(&mut ba, 512, 1024);
    let _ = reduce_with_limit(&mut ab, 512, 1024);
    let _ = reduce_with_limit(&mut bb, 512, 1024);

    aa.is_isomorphic_to(a)
        && ab.is_isomorphic_to(b)
        && ba.is_isomorphic_to(b)
        && bb.is_isomorphic_to(a)
}

fn not_xorset_test(a: &Term, b: &Term) -> bool {
    if a.is_isomorphic_to(b) {
        return false;
    }

    let mut aa = app(a.clone(), a.clone());
    let mut ab = app(a.clone(), b.clone());
    let mut ba = app(b.clone(), a.clone());
    let mut bb = app(b.clone(), b.clone());

    let _ = reduce_with_limit(&mut aa, 512, 1024);
    let _ = reduce_with_limit(&mut ba, 512, 1024);
    let _ = reduce_with_limit(&mut ab, 512, 1024);
    let _ = reduce_with_limit(&mut bb, 512, 1024);

    aa.is_isomorphic_to(b)
        && ab.is_isomorphic_to(b)
        && ba.is_isomorphic_to(b)
        && bb.is_isomorphic_to(a)
}

fn pairwise_compare<F>(terms: &[Term], test: &F, symmetric: bool) -> Option<(Term, Term)>
where
    F: Fn(&Term, &Term) -> bool,
{
    for (i, t1) in terms.iter().enumerate() {
        for (j, t2) in terms.iter().enumerate() {
            if test(t1, t2) {
                return Some((t1.clone(), t2.clone()));
            }
            if j >= i && symmetric {
                break;
            }
        }
    }
    None
}

async fn test_and_search_for_function<F>(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
    polling_interval: usize,
    test: F,
) -> (usize, Vec<Option<(Term, Term)>>)
where
    F: Fn(&Term, &Term) -> bool,
{
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
    soup.add_lambda_expressions(sample);
    let check_series =
        soup.simulate_and_poll_with_killer(run_length, polling_interval, false, |s| {
            let bests = s.k_most_frequent_exprs(10);
            let pairs = pairwise_compare(&bests, &test, false);
            (pairs.clone(), pairs.is_some())
        });
    (id, check_series)
}

pub fn look_for_xorset() {
    let mut gen = experiment_gen(config::ConfigSeed::new([0; 32]));
    let mut futures = FuturesUnordered::new();
    let run_length = 10000000;
    let polling_interval = 1000;
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(test_and_search_for_function(
            sample.into_iter(),
            i,
            run_length,
            polling_interval,
            xorset_test,
        )));
    }

    print!("Soup, ");
    println!();
    while let Some((id, series)) = block_on(futures.next()) {
        print!("{}, ", id);
        for i in series {
            if i.is_some() {
                print!("{:?}, ", i)
            }
        }
        println!();
    }
}

pub fn look_for_not_xorset() {
    let mut gen = experiment_gen(config::ConfigSeed::new([0; 32]));
    let mut futures = FuturesUnordered::new();
    let run_length = 10000000;
    let polling_interval = 1000;
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(test_and_search_for_function(
            sample.into_iter(),
            i,
            run_length,
            polling_interval,
            not_xorset_test,
        )));
    }

    print!("Soup, ");
    println!();
    while let Some((id, series)) = block_on(futures.next()) {
        print!("{}, ", id);
        for i in series {
            if i.is_some() {
                print!("{:?}, ", i)
            }
        }
        println!();
    }
}
