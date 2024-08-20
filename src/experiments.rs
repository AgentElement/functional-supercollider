use std::error::Error;

use async_std::task::spawn;
use clap::error::Result;
use futures::{stream::FuturesUnordered, StreamExt};
use lambda_calculus::{app, beta, reduction::Order::NOR, Term};
use plotters::prelude::*;

use crate::{config, generators::BTreeGen, soup::Soup};

fn xorset_test(a: Term, b: Term) -> bool {
    let aa = beta(app(a.clone(), a.clone()), NOR, 10000);
    let ab = beta(app(a.clone(), b.clone()), NOR, 10000);
    let ba = beta(app(b.clone(), a.clone()), NOR, 10000);
    let bb = beta(app(b.clone(), b.clone()), NOR, 10000);

    aa.is_isomorphic_to(&a)
        && ab.is_isomorphic_to(&b)
        && ba.is_isomorphic_to(&b)
        && bb.is_isomorphic_to(&a)
}

pub async fn look_for_xorset() {}

async fn simulate_soup(
    sample: impl Iterator<Item = Term>,
    id: usize,
    run_length: usize,
) -> (Soup, usize, f32) {
    let mut soup = Soup::from_config(&config::Reactor {
        rules: vec![String::from("\\x.\\y.x y")],
        discard_copy_actions: false,
        discard_identity: false,
        discard_free_variable_expressions: true,
        maintain_constant_population_size: true,
        discard_parents: false,
        reduction_cutoff: 512,
        size_cutoff: 1024,
        seed: config::ConfigSeed::new([0; 32]),
    });
    soup.perturb(sample);
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
    let mut soup = Soup::from_config(&config::Reactor {
        rules: vec![String::from("\\x.\\y.x y")],
        discard_copy_actions: false,
        discard_identity: false,
        discard_free_variable_expressions: true,
        maintain_constant_population_size: true,
        discard_parents: false,
        reduction_cutoff: 512,
        size_cutoff: 1024,
        seed: config::ConfigSeed::new([0; 32]),
    });
    soup.perturb(sample);
    let data = soup.simulate_and_poll(run_length, polling_interval, false, |s: &Soup| {
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
    let run_length = 1000000;
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
        let mut soup = Soup::from_config(&config::Reactor {
            rules: vec![String::from("\\x.\\y.x y")],
            discard_copy_actions: false,
            discard_identity: false,
            discard_free_variable_expressions: true,
            maintain_constant_population_size: true,
            discard_parents: false,
            reduction_cutoff: 512,
            size_cutoff: 1024,
            seed: config::ConfigSeed::new([0; 32]),
        });
        soup.perturb(sample);
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