use rand::{thread_rng, Rng};

use serde::{Deserialize, Serialize};

use crate::generators::Standardization;

use crate::utils::{decode_hex, encode_hex};

/// `Config` stores the global configuration of the program.
#[warn(missing_docs)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// The number of reactions to run for this simulation. Default: `100000`.
    pub run_limit: usize,

    /// The number of lambda expressions used to seed the generator. Default: `1000`
    pub sample_size: usize,

    /// Print out the state of the soup every `polling_interval` reactions. When set to `None`,
    /// never poll. Default: `None`.
    pub polling_interval: Option<usize>,

    /// When set, print out all logs for each individual reaction. Default: `false`.
    pub verbose_logging: bool,

    /// Configuration options for the random expression generator.
    pub generator_config: Generator,

    /// Configuration options for the lambda reactor.
    pub reactor_config: Reactor,
}

/// Configuration for the reactor
#[warn(missing_docs)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Reactor {
    /// Set of reaction rules. Each rule must always be a lambda expressions
    /// with two arguments. Default: `["\x.\y.x y"]`.
    pub rules: Vec<String>,

    /// When set, remove all results that are structurally isomorphic to parents.
    /// Default: `true`.
    pub discard_copy_actions: bool,

    /// When set, remove all results that are structurally isomorphic to the identity function:
    /// `\x.x`. Default: `true`.
    pub discard_identity: bool,

    /// When set, remove all expressions that contain free variables. Default: `true`.
    pub discard_free_variable_expressions: bool,

    /// When set, remove the parents from the soup instead of returning them. Default: `true`.
    pub discard_parents: bool,

    /// When set maintain a constant population size after each reaction. If there are more
    /// elements than the population originally started with, then remove elements randomly from
    /// the soup until the original population remains. If there are fewer elements after a
    /// reaction, then do nothing. This behavior may change. Default: `true`.
    pub maintain_constant_population_size: bool,

    ///  The number of reductions allowed before AlChemy gives up and fails the reaction. Default:
    ///  `500`.
    pub reduction_cutoff: usize,

    /// The largest size of any expression during a reduction step. Defaults to `1024`.
    pub size_cutoff: usize,

    /// The seed for the reactor. If set to `None`, then a seed is chosen
    /// randomly. Default: `None`
    pub seed: ConfigSeed,
}

/// Configuration for the generators
#[warn(missing_docs)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Generator {
    /// Use the btree generator
    BTree(BTreeGen),

    /// Use Fontana's generator
    Fontana(FontanaGen),
}

pub trait GenConfig {
    fn new() -> Self;
}

/// Configuration for the BTree generator
#[warn(missing_docs)]
#[derive(Serialize, Deserialize, Debug)]
pub struct BTreeGen {
    /// The seed for the lambda expression generator. If set to `None`, then a seed is chosen
    /// randomly. Default: `None`
    pub seed: ConfigSeed,

    /// Number of nodes in the binary tree
    pub size: u32,

    /// Probability that a leaf vertex is a free variable
    pub freevar_generation_probability: f64,

    /// Size of the variable palette
    pub n_max_free_vars: u32,

    /// Standardization scheme. Defaults to prefix standardization (this is different from the
    /// paper!)
    pub standardization: Standardization,
}

#[warn(missing_docs)]
#[derive(Serialize, Deserialize, Debug)]
/// Generator-specific configuration derived from Walter Fontana's original scheme.
pub struct FontanaGen {
    /// The seed for the lambda expression generator. If set to `None`, then a seed is chosen
    /// randomly. Default: `None`
    pub seed: ConfigSeed,

    /// Minimum depth before leaf nodes can be generated
    pub min_depth: u32,

    /// Maximum depth of the generated trees
    pub max_depth: u32,

    /// Probability range of an abstraction being generated. Linearly changes from start to end,
    /// varying with depth
    pub abstraction_prob_range: (f64, f64),

    /// Probability range of an application being generated. Linearly changes from start to end,
    /// varying with depth
    pub application_prob_range: (f64, f64),
}

impl GenConfig for FontanaGen {
    fn new() -> Self {
        FontanaGen {
            seed: ConfigSeed(None),
            min_depth: 0,
            max_depth: 10,
            application_prob_range: (0.5, 0.3),
            abstraction_prob_range: (0.3, 0.5),
        }
    }
}

impl Reactor {
    /// Produce a new `ReactorConfig` struct with default values.
    pub fn new() -> Self {
        Reactor {
            rules: vec![String::from("\\x.\\y.x y")],

            discard_copy_actions: true,
            discard_identity: true,
            discard_free_variable_expressions: true,
            maintain_constant_population_size: true,
            discard_parents: false,
            reduction_cutoff: 500,
            size_cutoff: 500,
            seed: ConfigSeed(None),
        }
    }
}

// TODO: Eventually, all config objects will use `default` instead of `new`. For now, this just
// fixes a clippy lint
impl Default for Reactor {
    fn default() -> Self {
        Reactor::new()
    }
}

impl GenConfig for BTreeGen {
    /// Produce a new `BTreeGenConfig` struct with default values.
    fn new() -> Self {
        BTreeGen {
            size: 20,
            freevar_generation_probability: 0.2,
            standardization: Standardization::Prefix,
            n_max_free_vars: 6,
            seed: ConfigSeed(None),
        }
    }
}

impl Config {
    /// Create a config object from a string
    pub fn from_config_str(s: &str) -> Config {
        serde_json::from_str(s).unwrap()
    }

    /// Convert the config object to a string
    pub fn to_config_str(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Produce a new `Config` struct with default values.
    pub fn new() -> Self {
        Config {
            reactor_config: Reactor::new(),
            generator_config: Generator::BTree(BTreeGen::new()),
            run_limit: 100000,
            sample_size: 1000,
            polling_interval: None,
            verbose_logging: false,
        }
    }

    pub fn set_reduction_cutoff(&mut self, cutoff: usize) {
        self.reactor_config.reduction_cutoff = cutoff;
    }

    pub fn set_run_limit(&mut self, limit: usize) {
        self.run_limit = limit;
    }

    pub fn set_polling_interval(&mut self, interval: Option<usize>) {
        self.polling_interval = interval;
    }

    pub fn set_verbose_logging(&mut self, logging: bool) {
        self.verbose_logging = logging;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a seed for serde RNGs in the configuration file. Mostly here because we want
/// to ser/de to/from a hex string.
#[warn(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigSeed(Option<[u8; 32]>);

impl ConfigSeed {
    /// Get the seed item
    pub fn get(&self) -> [u8; 32] {
        self.0.unwrap_or(thread_rng().gen())
    }

    pub fn seed(&self) -> Option<[u8; 32]> {
        self.0
    }

    pub fn new(seed: [u8; 32]) -> Self {
        ConfigSeed(Some(seed))
    }

    pub fn blank() -> Self {
        ConfigSeed(None)
    }
}

/// Manually serialize [u8; 32] to a hex string
impl Serialize for ConfigSeed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(seed) = self.seed() {
            serializer.serialize_str(&encode_hex(&seed))
        } else {
            serializer.serialize_none()
        }
    }
}

/// Manually deserialize a hex string to [u8; 32]
///
/// SAFETY: `panic!`s when hex string is malformed or of odd length
impl<'de> Deserialize<'de> for ConfigSeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seed_string: Option<&str> = serde::de::Deserialize::deserialize(deserializer)?;
        Ok(if let Some(s) = seed_string {
            let hexvec = decode_hex(s).unwrap();
            ConfigSeed::new(hexvec.try_into().unwrap())
        } else {
            ConfigSeed::blank()
        })
    }
}
