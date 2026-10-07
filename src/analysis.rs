use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::lambda::recursive::LambdaSoup;
use crate::utils::HeapObject;

use lambda_calculus::Term;

impl LambdaSoup {
    // This is expensive, quadratic in the number of expressions. It can
    // probably be written to be faster, but it's not a bottleneck right now.
    pub fn unique_expressions(&self) -> HashSet<Term> {
        HashSet::<Term>::from_iter(self.lambda_expressions().cloned())
    }

    pub fn expression_counts(&self) -> HashMap<Term, u32> {
        let mut map = HashMap::<Term, u32>::new();
        for expr in self.lambda_expressions().cloned() {
            *map.entry(expr).or_default() += 1
        }
        map
    }

    // The use of HeapObject is a code smell, refactor later
    pub fn k_most_frequent_exprs(&self, k: usize) -> Vec<Term> {
        let mut map = HashMap::<&Term, u32>::new();
        for x in self.lambda_expressions() {
            *map.entry(x).or_default() += 1;
        }

        let mut heap = BinaryHeap::with_capacity(k + 1);
        for (x, count) in map.into_iter() {
            heap.push(Reverse(HeapObject::new(count, x)));
            if heap.len() > k {
                heap.pop();
            }
        }
        heap.into_sorted_vec()
            .into_iter()
            .map(|r| {
                let tup = r.0.to_tuple();
                tup.1.clone()
            })
            .collect()
    }

    pub fn population_entropy(&self) -> f32 {
        let mut entropy = 0.0;
        let n = self.len() as f32;
        for (_, value) in self.expression_counts().iter() {
            let pi = (*value as f32) / n;
            entropy -= pi * pi.log10();
        }
        entropy
    }

    pub fn jacard_index(&self, other: &LambdaSoup) -> f32 {
        let selfcounts = self.expression_counts();
        let othercounts = other.expression_counts();

        let mut intersection: u32 = 0;
        for (k, v) in selfcounts {
            if let Some(c) = othercounts.get(&k) {
                intersection += c.min(&v);
            }
        }
        // Jaccard for multisets: |A ∩ B| / |A ∪ B|,
        // where ∩ is sum of mins and ∪ is sum of maxes.
        // Using identity: sum(max) = |A| + |B| - sum(min).
        let union = self.len() + other.len() - intersection as usize;
        if union == 0 {
            return 1.0; // both empty → treat as identical
        }
        (intersection as f32) / (union as f32)
    }
}
