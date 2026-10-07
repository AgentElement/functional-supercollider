// src/python.rs
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use rand::Rng;
use serde::{Deserialize, Serialize};

use lambda_calculus::{parse, term::Notation::Classic};

use crate::config::{self, ConfigSeed, Reactor as RustReactor};
use crate::generators::{
    BTreeGen as RustBTreeGen, FontanaGen as RustFontanaGen, Standardization as RustStandardization,
};
use crate::lambda::recursive::{
    AlchemyCollider, LambdaCollisionError, LambdaCollisionOk, LambdaParticle,
};
use crate::supercollider::Soup as GenericSoup;
use crate::utils::{decode_hex, encode_hex};

// Concrete soup alias for the recursive lambda flavor
type RustSoup =
    GenericSoup<LambdaParticle, AlchemyCollider, LambdaCollisionOk, LambdaCollisionError>;

// ============ Helper for Seed Parsing ============

fn parse_seed(seed_hex: Option<String>) -> PyResult<[u8; 32]> {
    match seed_hex {
        Some(s) => {
            let bytes = decode_hex(&s).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid hex seed: {}", e))
            })?;
            bytes.try_into().map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(
                    "Seed must be exactly 32 bytes (64 hex chars)",
                )
            })
        }
        None => {
            let mut rng = rand::thread_rng();
            Ok(rng.gen())
        }
    }
}

// ============ Errors exposed to Python ============

#[pyclass]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PyReactionError {
    kind: ReactionErrorKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ReactionErrorKind {
    ExceedsReductionLimit,
    NotEnoughExpressions,
    IsIdentity,
    IsParent,
    HasFreeVariables,
    ExceedsDepthLimit,
    RecursiveArgument,
    BadArgument,
}

impl ReactionErrorKind {
    fn as_str(&self) -> &'static str {
        match self {
            ReactionErrorKind::ExceedsReductionLimit => "exceeds_reduction_limit",
            ReactionErrorKind::NotEnoughExpressions => "not_enough_expressions",
            ReactionErrorKind::IsIdentity => "is_identity",
            ReactionErrorKind::IsParent => "is_parent",
            ReactionErrorKind::HasFreeVariables => "has_free_variables",
            ReactionErrorKind::ExceedsDepthLimit => "exceeds_depth_limit",
            ReactionErrorKind::RecursiveArgument => "recursive_argument",
            ReactionErrorKind::BadArgument => "bad_argument",
        }
    }
}

#[pymethods]
impl PyReactionError {
    #[getter]
    fn kind(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl From<LambdaCollisionError> for PyReactionError {
    fn from(error: LambdaCollisionError) -> Self {
        let kind = match error {
            LambdaCollisionError::ExceedsReductionLimit => ReactionErrorKind::ExceedsReductionLimit,
            LambdaCollisionError::NotEnoughExpressions => ReactionErrorKind::NotEnoughExpressions,
            LambdaCollisionError::IsIdentity => ReactionErrorKind::IsIdentity,
            LambdaCollisionError::IsParent => ReactionErrorKind::IsParent,
            LambdaCollisionError::HasFreeVariables => ReactionErrorKind::HasFreeVariables,
            LambdaCollisionError::ExceedsDepthLimit => ReactionErrorKind::ExceedsDepthLimit,
            LambdaCollisionError::RecursiveArgument => ReactionErrorKind::RecursiveArgument,
            LambdaCollisionError::BadArgument => ReactionErrorKind::BadArgument,
        };
        PyReactionError { kind }
    }
}

// ============ Reaction Record exposed to Python ============

#[pyclass]
#[derive(Clone)]
pub struct PyReactionRecord {
    #[pyo3(get)]
    pub step: usize,
    #[pyo3(get)]
    pub left: String,
    #[pyo3(get)]
    pub right: String,
    #[pyo3(get)]
    pub products: Vec<String>,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl PyReactionRecord {
    fn __repr__(&self) -> String {
        if self.success {
            format!(
                "ReactionRecord(step={}, left=..., right=..., products=[{}])",
                self.step,
                self.products.len()
            )
        } else {
            format!(
                "ReactionRecord(step={}, FAILED: {})",
                self.step,
                self.error.as_deref().unwrap_or("unknown")
            )
        }
    }
}

// ============ Reactor wrapper ============

#[pyclass]
pub struct PyReactor {
    pub(crate) inner: RustReactor,
}

// Reactor config with an optional hex seed. With no seed, a random one is
// chosen here and stored, so it can be read back and the run reproduced.
fn reactor_with_seed(seed: Option<String>) -> PyResult<RustReactor> {
    let mut cfg = RustReactor::new();
    cfg.seed = ConfigSeed::new(parse_seed(seed)?);
    Ok(cfg)
}

#[pymethods]
impl PyReactor {
    #[new]
    #[pyo3(signature = (seed=None))]
    fn new(seed: Option<String>) -> PyResult<Self> {
        Ok(PyReactor {
            inner: reactor_with_seed(seed)?,
        })
    }

    /// The seed this reactor uses, as a 64-char hex string.
    #[getter]
    fn seed(&self) -> String {
        encode_hex(&self.inner.seed.get())
    }
}

// ============ Standardization wrapper ============

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyStandardization {
    standardization: RustStandardization,
}

#[pymethods]
impl PyStandardization {
    #[new]
    fn new(kind: &str) -> PyResult<Self> {
        let standardization = match kind {
            "prefix" => RustStandardization::Prefix,
            "postfix" => RustStandardization::Postfix,
            "none" => RustStandardization::None,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid standardization type",
                ))
            }
        };
        Ok(PyStandardization { standardization })
    }
}

impl From<PyStandardization> for RustStandardization {
    fn from(py_std: PyStandardization) -> Self {
        py_std.standardization
    }
}

// ============ Soup wrapper ============

#[pyclass]
pub struct PySoup {
    inner: RustSoup,
    seed: [u8; 32],
}

#[pymethods]
impl PySoup {
    #[new]
    #[pyo3(signature = (seed=None))]
    fn new(seed: Option<String>) -> PyResult<Self> {
        let cfg = reactor_with_seed(seed)?;
        Ok(PySoup {
            inner: RustSoup::from_config(&cfg),
            seed: cfg.seed.get(),
        })
    }

    #[staticmethod]
    fn from_config(cfg: &PyReactor) -> Self {
        // A PyReactor always holds a resolved seed (see reactor_with_seed),
        // so get() returns the same seed the soup is built with.
        PySoup {
            inner: RustSoup::from_config(&cfg.inner),
            seed: cfg.inner.seed.get(),
        }
    }

    /// The seed this soup's collisions use, as a 64-char hex string.
    /// Pass it back as `PySoup(seed=...)` to reproduce the run.
    #[getter]
    fn seed(&self) -> String {
        encode_hex(&self.seed)
    }

    fn perturb(&mut self, expressions: Vec<String>) {
        let terms = expressions
            .into_iter()
            .filter_map(|s| parse(&s, Classic).ok());
        self.inner.add_lambda_expressions(terms);
    }

    fn simulate_for(&mut self, n: usize, log: bool) -> usize {
        self.inner.simulate_for(n, log)
    }

    /// Simulate for `n` collisions and return a list of ReactionRecord
    /// objects capturing every reaction (parents + products).
    fn simulate_for_logged(&mut self, n: usize) -> Vec<PyReactionRecord> {
        self.inner
            .simulate_for_logged(n)
            .into_iter()
            .map(|r| PyReactionRecord {
                step: r.step,
                left: r.left.to_string(),
                right: r.right.to_string(),
                products: r.products.iter().map(|p| p.to_string()).collect(),
                success: r.success,
                error: r.error,
            })
            .collect()
    }

    /// Simulate for `n` collisions and return only successful reaction records.
    /// Uses less memory than `simulate_for_logged` for large n.
    fn simulate_for_logged_filtered(&mut self, n: usize) -> Vec<PyReactionRecord> {
        self.inner
            .simulate_for_logged_filtered(n)
            .into_iter()
            .map(|r| PyReactionRecord {
                step: r.step,
                left: r.left.to_string(),
                right: r.right.to_string(),
                products: r.products.iter().map(|p| p.to_string()).collect(),
                success: r.success,
                error: r.error,
            })
            .collect()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
    fn collisions(&self) -> usize {
        self.inner.collisions()
    }

    fn expressions(&self) -> Vec<String> {
        self.inner
            .lambda_expressions()
            .map(|t| t.to_string())
            .collect()
    }
    fn unique_expressions(&self) -> Vec<String> {
        self.inner
            .unique_expressions()
            .into_iter()
            .map(|t| t.to_string())
            .collect()
    }
    fn expression_counts(&self) -> Vec<(String, u32)> {
        self.inner
            .expression_counts()
            .into_iter()
            .map(|(t, c)| (t.to_string(), c))
            .collect()
    }
    fn population_entropy(&self) -> f32 {
        self.inner.population_entropy()
    }
}

// ============ Generators ============

#[pyclass]
pub struct PyBTreeGen {
    inner: RustBTreeGen,
}

#[pymethods]
impl PyBTreeGen {
    #[new]
    fn new() -> Self {
        PyBTreeGen {
            inner: RustBTreeGen::new(),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (size, freevar_generation_probability, max_free_vars, std, seed=None))]
    fn from_config(
        size: u32,
        freevar_generation_probability: f64,
        max_free_vars: u32,
        std: PyStandardization,
        seed: Option<String>,
    ) -> PyResult<Self> {
        let seed_bytes = parse_seed(seed)?;

        let cfg = config::BTreeGen {
            size,
            freevar_generation_probability,
            n_max_free_vars: max_free_vars,
            standardization: std.into(),
            seed: ConfigSeed::new(seed_bytes),
        };
        Ok(PyBTreeGen {
            inner: RustBTreeGen::from_config(&cfg),
        })
    }

    /// The seed this generator uses, as a 64-char hex string.
    #[getter]
    fn seed(&self) -> String {
        encode_hex(&self.inner.seed())
    }

    fn generate(&mut self) -> String {
        self.inner.generate().to_string()
    }

    #[pyo3(signature = (n, unique=false))]
    fn generate_n(&mut self, n: usize, unique: bool) -> Vec<String> {
        if unique {
            self.inner
                .generate_n_unique(n)
                .into_iter()
                .map(|t| t.to_string())
                .collect()
        } else {
            self.inner
                .generate_n(n)
                .into_iter()
                .map(|t| t.to_string())
                .collect()
        }
    }
}

#[pyclass]
pub struct PyFontanaGen {
    inner: RustFontanaGen,
}

#[pymethods]
impl PyFontanaGen {
    /// Build a Fontana generator from config values.
    /// No free variable parameters — FontanaGen only produces closed terms.
    #[staticmethod]
    #[pyo3(signature = (abs_range, app_range, min_depth, max_depth, seed=None))]
    pub fn from_config(
        abs_range: (f64, f64),
        app_range: (f64, f64),
        min_depth: u32,
        max_depth: u32,
        seed: Option<String>,
    ) -> PyResult<Self> {
        let seed_bytes = parse_seed(seed)?;

        let cfg = config::FontanaGen {
            abstraction_prob_range: abs_range,
            application_prob_range: app_range,
            min_depth,
            max_depth,
            seed: ConfigSeed::new(seed_bytes),
        };
        Ok(PyFontanaGen {
            inner: RustFontanaGen::from_config(&cfg),
        })
    }

    /// The seed this generator uses, as a 64-char hex string.
    #[getter]
    fn seed(&self) -> String {
        encode_hex(&self.inner.seed())
    }

    /// Generate a single lambda term
    pub fn generate(&mut self) -> String {
        self.inner.generate().to_string()
    }

    /// Convenience: generate N terms
    #[pyo3(signature = (n, unique=false))]
    pub fn generate_n(&mut self, n: usize, unique: bool) -> Vec<String> {
        if unique {
            self.inner
                .generate_n_unique(n)
                .into_iter()
                .map(|t| t.to_string())
                .collect()
        } else {
            self.inner
                .generate_n(n)
                .into_iter()
                .map(|t| t.to_string())
                .collect()
        }
    }
}

// ============ Utilities ============

#[allow(clippy::useless_conversion)]
#[pyfunction]
fn decode_hex_py(hex_string: &str) -> PyResult<Vec<u8>> {
    decode_hex(hex_string).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyfunction]
fn encode_hex_py(bytes: Vec<u8>) -> String {
    encode_hex(&bytes)
}

// ============ Public registration hook ============

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySoup>()?;
    m.add_class::<PyReactor>()?;
    m.add_class::<PyReactionError>()?;
    m.add_class::<PyReactionRecord>()?;
    m.add_class::<PyStandardization>()?;
    m.add_class::<PyBTreeGen>()?;
    m.add_class::<PyFontanaGen>()?;
    m.add_function(wrap_pyfunction!(decode_hex_py, m)?)?;
    m.add_function(wrap_pyfunction!(encode_hex_py, m)?)?;
    Ok(())
}
