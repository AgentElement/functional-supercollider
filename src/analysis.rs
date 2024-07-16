use std::collections::{HashMap, HashSet};

use crate::soup::Soup;
use crate::soup::Tape;

use lambda_calculus::Term;

struct Property {
    n: usize,
    rhs: Vec<usize>,
}


impl Soup {
    // This is expensive, quadratic in the number of expressions. It can
    // probably be written to be faster, but it's not a bottleneck right now.
    pub fn unique_expressions(&self) -> HashSet<Term> {
        HashSet::<Term>::from_iter(self.expressions().cloned())
    }

    pub fn expression_counts(&self) -> HashMap<Term, u32> {
        let mut map = HashMap::<Term, u32>::new();
        for expr in self.expressions().cloned() {
            map.entry(expr).and_modify(|e| *e += 1).or_insert(1);
        }
        map
    }

    fn find_functions_with_property(&self, property: &Property) {}

    pub fn population_entropy(&self) -> f32 {
        let mut entropy = 0.0;
        let n = self.len() as f32;
        for (_, value) in self.expression_counts().iter() {
            let pi = (*value as f32) / n;
            entropy -= pi * pi.log10();
        }
        entropy
    }
}
