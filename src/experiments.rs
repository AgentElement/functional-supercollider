use std::collections::HashMap;
use std::error::Error;

use async_std::task::spawn;
use clap::error::Result;
use futures::{stream::FuturesUnordered, StreamExt};
use lambda_calculus::{
    abs, app,
    data::{
        boolean::{self, and},
        num::church::{add, eq},
    },
    parse,
    term::Notation::Classic,
    IntoChurchNum,
    Term::{self, Abs},
};
use plotters::prelude::*;
use rand::random;

use crate::{
    config::{self, ConfigSeed},
    generators::BTreeGen,
    lambda::{reduce_with_limit, LambdaSoup},
    read_inputs,
};

pub fn experiment_soup(seed: ConfigSeed) -> LambdaSoup {
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

pub fn test_add(a: usize, b: usize) -> Term {
    let mut test = parse(r"\eq. \a. \b. \ab. \f. (eq (f a b) ab)", Classic).unwrap();
    test = app!(
        test,
        eq(),
        a.into_church(),
        b.into_church(),
        (a + b).into_church()
    );
    // `test` has type (church -> church -> church) -> bool
    test.reduce(lambda_calculus::NOR, 0);
    test
}

pub fn test_add_seq(pairs: impl Iterator<Item = (usize, usize)>) -> Term {
    let mut test = parse(r"\f. \a. \b. a", Classic).unwrap();
    for (u, v) in pairs {
        let gut = parse(
            r"\and. \test. \testadd. \f. and (test f) (testadd f)",
            Classic,
        )
        .unwrap();
        test = app!(gut, and(), test, test_add(u, v));
    }
    test.reduce(lambda_calculus::NOR, 0);
    let mut comp = app!(test.clone(), add());
    comp.reduce(lambda_calculus::NOR, 0);
    assert!(comp.is_isomorphic_to(&boolean::tru()));
    test
}

pub fn test_succ(a: usize) -> Term {
    let mut test = parse(r"\eq. \a. \asucc. \f. (eq (f a) asucc)", Classic).unwrap();
    test = app!(test, eq(), a.into_church(), (a + 1).into_church());
    // `test` has type (church -> church) -> bool
    test.reduce(lambda_calculus::NOR, 0);
    test
}

pub fn test_sub(a: usize, b: usize) -> Term {
    let mut test = parse(r"\eq. \a. \b. \ab. \f. (eq (f a b) ab)", Classic).unwrap();
    test = app!(
        test,
        eq(),
        a.into_church(),
        b.into_church(),
        (a - b).into_church()
    );
    // `test` has type (church -> church -> church) -> bool
    test.reduce(lambda_calculus::NOR, 0);
    test
}

pub fn test_pred(a: usize) -> Term {
    let mut test = parse(r"\eq. \a. \apred. \f. (eq (f a) apred)", Classic).unwrap();
    test = app!(test, eq(), a.into_church(), (a - 1).into_church());
    // `test` has type (church -> church) -> bool
    test.reduce(lambda_calculus::NOR, 0);
    test
}

pub fn test_add_reduction() -> Term {
    let mut comp = app!(test_add(20, 20), add());
    let n = comp.reduce(lambda_calculus::NOR, 0);
    println!("add reduction in {n} steps: {comp}");
    comp
}


pub async fn add_search_with_test() {
    let mut futures = FuturesUnordered::new();
    let run_length = 1000000;
    let polling_interval = 1000;
    let sample = read_inputs().collect::<Vec<Term>>();
    for i in 0..100 {
        let distribution = sample.clone().into_iter().cycle().take(4000);
        let tests = (0..1000).map(|_| {
            let test = test_add_seq((0..5).map(|_| {
                let u = random::<usize>() % 20;
                let v = random::<usize>() % 20;
                (u, v)
            }));
            test
        });
        futures.push(spawn(add_magic_tests(
            distribution,
            tests,
            i,
            run_length,
            polling_interval,
        )));
    }

    print!("Soup, ");
    println!();
    while let Some((id, series)) = futures.next().await {
        print!("{}, ", id);
        for i in series {
            print!("{:?}, ", i)
        }
        println!();
    }
}

async fn add_magic_tests(
    sample: impl Iterator<Item = Term>,
    tests: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
    polling_interval: usize,
) -> (usize, Vec<(usize, usize)>) {
    let mut soup = experiment_soup(ConfigSeed::new([1; 32]));
    soup.add_lambda_expressions(sample);
    soup.add_test_expressions(tests);
    let mut populations = Vec::new();
    for _ in 0..100 {
        let pops = soup.simulate_and_poll(run_length, polling_interval / 100, false, |s| {
            (
                s.expressions().filter(|e| e.is_recursive()).count(),
                s.population_of(&add()),
            )
        });
        populations.extend(pops);
        let n_remaining = 1000 - soup.expressions().filter(|e| e.is_recursive()).count();
        let tests = (0..n_remaining).map(|_| {
            let test = test_add_seq((0..5).map(|_| {
                let u = random::<usize>() % 20;
                let v = random::<usize>() % 20;
                (u, v)
            }));
            test
        });
        soup.add_test_expressions(tests);
    }
    (id, populations)
}

async fn simulate_additive_murder(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
    polling_interval: usize,
) -> (usize, Vec<usize>) {
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
    soup.add_lambda_expressions(sample);
    let add = parse(
        r"\m.\n. m ((\m.\n. m (\n.\x.\y. x (n x y)) n) n) (\x.\y.y)",
        Classic,
    )
    .unwrap();
    let check_series =
        soup.simulate_and_poll_with_killer(run_length, polling_interval, false, |s| {
            (
                s.collisions(),
                s.expressions()
                    .any(|e| e.get_underlying_term().is_isomorphic_to(&add)),
            )
        });
    (id, check_series)
}

pub async fn look_for_add() {
    let mut futures = FuturesUnordered::new();
    let run_length = 1000000;
    let polling_interval = 1000;
    let sample = read_inputs().collect::<Vec<Term>>();
    for i in 0..1000 {
        futures.push(spawn(simulate_additive_murder(
            sample.clone().into_iter().cycle().take(10000),
            i,
            run_length,
            polling_interval,
        )));
    }

    print!("Soup, ");
    println!();
    while let Some((id, series)) = futures.next().await {
        print!("{}, ", id);
        for i in series {
            print!("{:?}, ", i)
        }
        println!();
    }
}

pub fn one_sample_with_dist() {
    let run_length = 1000000;
    let polling_interval = 1000;
    let polls = run_length / polling_interval;
    let sample = read_inputs().collect::<Vec<Term>>();
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));

    soup.add_lambda_expressions(sample.into_iter().cycle().take(10000));
    let counts = soup.simulate_and_poll(run_length, polling_interval, false, |s| {
        s.expression_counts()
    });

    let mut map = HashMap::<Term, Vec<u32>>::new();
    for (i, count) in counts.iter().enumerate() {
        for (term, val) in count.iter() {
            map.entry(term.clone())
                .or_insert(vec![0; i.try_into().unwrap()])
                .push(*val);
        }
        for (term, vals) in map.iter_mut() {
            if !count.contains_key(term) {
                vals.push(0);
            }
        }
    }

    print!("Term, ");
    for i in 0..polls {
        print!("{}, ", i)
    }
    println!();
    for (term, vec) in map.iter() {
        print!("{}, ", term);
        for c in vec {
            print!("{}, ", c);
        }
        println!();
    }
}

pub async fn simulate_sample() {
    let mut futures = FuturesUnordered::new();
    let run_length = 1000000;
    let polling_interval = 1000;
    let polls = run_length / polling_interval;
    let sample = read_inputs().collect::<Vec<Term>>();
    for i in 0..1000 {
        futures.push(spawn(simulate_soup_and_produce_entropies(
            sample.clone().into_iter().cycle().take(10000),
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
    while let Some((id, data)) = futures.next().await {
        print!("{}, ", id);
        for i in data {
            print!("{}, ", i)
        }
        println!();
    }
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

fn pairwise_compare<F>(terms: &[Term], test: F, symmetric: bool) -> Option<(Term, Term)>
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

async fn simulate_soup_murder(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
    polling_interval: usize,
) -> (usize, Vec<Option<(Term, Term)>>) {
    let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
    soup.add_lambda_expressions(sample);
    let check_series =
        soup.simulate_and_poll_with_killer(run_length, polling_interval, false, |s| {
            let bests = s.k_most_frequent_exprs(10);
            let pairs = pairwise_compare(&bests, not_xorset_test, false);
            (pairs.clone(), pairs.is_some())
        });
    (id, check_series)
}

pub async fn look_for_xorset() {
    let mut gen = BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed: config::ConfigSeed::new([0; 32]),
    });
    let mut futures = FuturesUnordered::new();
    let run_length = 10000000;
    let polling_interval = 1000;
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(simulate_soup_murder(
            sample.into_iter(),
            i,
            run_length,
            polling_interval,
        )));
    }

    print!("Soup, ");
    println!();
    while let Some((id, series)) = futures.next().await {
        print!("{}, ", id);
        for i in series {
            if i.is_some() {
                print!("{:?}, ", i)
            }
        }
        println!();
    }
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

pub async fn entropy_series() {
    let mut gen = BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed: config::ConfigSeed::new([0; 32]),
    });
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
    while let Some((id, data)) = futures.next().await {
        print!("{}, ", id);
        for i in data {
            print!("{}, ", i)
        }
        println!();
    }
}

pub async fn entropy_test() {
    let mut gen = BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed: config::ConfigSeed::new([0; 32]),
    });
    let mut futures = FuturesUnordered::new();
    for i in 0..1000 {
        let sample = gen.generate_n(10000);
        futures.push(spawn(simulate_soup(sample.into_iter(), i, 10000000)));
    }

    let mut data = Vec::new();
    println!("Soup, Entropy, Failure rate");
    while let Some((soup, id, failure_rate)) = futures.next().await {
        let entropy = soup.population_entropy();
        println!("{}, {}, {}", id, entropy, failure_rate);
        data.push(entropy);
    }

    plot_histogram(&data).unwrap();
}

pub fn sync_entropy_test() {
    let mut gen = BTreeGen::from_config(&config::BTreeGen {
        size: 20,
        freevar_generation_probability: 0.2,
        standardization: crate::generators::Standardization::Prefix,
        n_max_free_vars: 6,
        seed: config::ConfigSeed::new([0; 32]),
    });

    for i in 0..100 {
        let sample = gen.generate_n(1000);
        let mut soup = experiment_soup(ConfigSeed::new([0; 32]));
        soup.add_lambda_expressions(sample);
        soup.simulate_for(100000, false);
        let entropy = soup.population_entropy();
        println!("{}: {}", i, entropy);
    }
}

fn plot_histogram(data: &[f32]) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::new("test.png", (1000, 1000)).into_drawing_area();
    root.fill(&BLACK)?;

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("Population Entropy", ("sans-serif", 50.0))
        .build_cartesian_2d((0u32..10u32).into_segmented(), 0f32..3f32)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .y_desc("Count")
        .x_desc("Bucket")
        .axis_desc_style(("sans-serif", 15))
        .draw()?;

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(RED.mix(0.5).filled())
            .data(data.iter().map(|x: &f32| (1, *x))),
    )?;

    root.present().expect("Unable to write result to file");
    Ok(())
}
