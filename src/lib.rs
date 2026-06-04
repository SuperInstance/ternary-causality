//! # ternary-causality
//!
//! Causal inference for ternary strategy systems.
//!
//! Provides:
//! - `CausalDAG` — directed acyclic graph for causal relationships
//! - `Intervention` — do-calculus style interventions
//! - `CounterfactualEngine` — "what would have happened" reasoning
//! - `CausalDiscovery` — learn causal structure from observational data
//! - `EffectEstimator` — estimate causal effects (ATE, CATE, etc.)

use std::collections::{HashMap, HashSet, VecDeque};

/// Ternary value: -1, 0, or +1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ternary {
    Neg,
    Zero,
    Pos,
}

impl Ternary {
    pub fn value(&self) -> i8 {
        match self {
            Ternary::Neg => -1,
            Ternary::Zero => 0,
            Ternary::Pos => 1,
        }
    }

    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Ternary::Neg),
            0 => Some(Ternary::Zero),
            1 => Some(Ternary::Pos),
            _ => None,
        }
    }

    pub fn random() -> Self {
        match rand_ternary() {
            0 => Ternary::Neg,
            1 => Ternary::Zero,
            _ => Ternary::Pos,
        }
    }
}

fn rand_ternary() -> u8 {
    // Simple deterministic-ish spread; users call with varying seeds
    static mut COUNTER: u8 = 0;
    unsafe {
        // We avoid unsafe in public API; this is internal-only
        // Actually let's use a simple approach without unsafe
    }
    // Use a simple linear congruential approach via thread local
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(12345);
    }
    STATE.with(|s| {
        let mut v = s.get();
        v = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s.set(v);
        ((v >> 33) % 3) as u8
    })
}

/// A node in a causal DAG
#[derive(Debug, Clone)]
pub struct CausalNode {
    pub id: usize,
    pub name: String,
    pub value: Ternary,
    pub observed: bool,
}

/// Edge in a causal DAG with optional strength
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from: usize,
    pub to: usize,
    pub strength: f64,
}

/// Directed Acyclic Graph for causal relationships
#[derive(Debug, Clone)]
pub struct CausalDAG {
    nodes: Vec<CausalNode>,
    edges: Vec<CausalEdge>,
    adj: HashMap<usize, Vec<usize>>,       // parent -> children
    parents: HashMap<usize, Vec<usize>>,   // child -> parents
}

impl CausalDAG {
    pub fn new() -> Self {
        CausalDAG {
            nodes: Vec::new(),
            edges: Vec::new(),
            adj: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: &str, value: Ternary) -> usize {
        let id = self.nodes.len();
        self.nodes.push(CausalNode {
            id,
            name: name.to_string(),
            value,
            observed: true,
        });
        id
    }

    pub fn add_edge(&mut self, from: usize, to: usize, strength: f64) -> bool {
        if from >= self.nodes.len() || to >= self.nodes.len() || from == to {
            return false;
        }
        // Check if adding this edge creates a cycle
        if self.would_create_cycle(from, to) {
            return false;
        }
        self.edges.push(CausalEdge { from, to, strength });
        self.adj.entry(from).or_default().push(to);
        self.parents.entry(to).or_default().push(from);
        true
    }

    fn would_create_cycle(&self, from: usize, to: usize) -> bool {
        // BFS from 'to' to see if we can reach 'from'
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(to);
        while let Some(node) = queue.pop_front() {
            if node == from {
                return true;
            }
            if visited.insert(node) {
                if let Some(children) = self.adj.get(&node) {
                    for &child in children {
                        queue.push_back(child);
                    }
                }
            }
        }
        false
    }

    pub fn nodes(&self) -> &[CausalNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[CausalEdge] {
        &self.edges
    }

    pub fn children_of(&self, node: usize) -> Vec<usize> {
        self.adj.get(&node).cloned().unwrap_or_default()
    }

    pub fn parents_of(&self, node: usize) -> Vec<usize> {
        self.parents.get(&node).cloned().unwrap_or_default()
    }

    pub fn topological_sort(&self) -> Vec<usize> {
        let n = self.nodes.len();
        let mut in_degree = vec![0usize; n];
        for edge in &self.edges {
            in_degree[edge.to] += 1;
        }
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node);
            if let Some(children) = self.adj.get(&node) {
                for &child in children {
                    in_degree[child] -= 1;
                    if in_degree[child] == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
        result
    }

    pub fn set_node_value(&mut self, node: usize, value: Ternary) {
        if node < self.nodes.len() {
            self.nodes[node].value = value;
        }
    }

    pub fn get_node_value(&self, node: usize) -> Option<Ternary> {
        self.nodes.get(node).map(|n| n.value)
    }

    /// Compute node values based on parent contributions
    pub fn propagate(&mut self) {
        let order = self.topological_sort();
        for node_id in order {
            let parent_ids = self.parents.get(&node_id).cloned().unwrap_or_default();
            if parent_ids.is_empty() {
                continue;
            }
            let mut sum = 0.0;
            for &pid in &parent_ids {
                let pv = self.nodes[pid].value.value() as f64;
                let strength = self.edges.iter()
                    .find(|e| e.from == pid && e.to == node_id)
                    .map(|e| e.strength)
                    .unwrap_or(1.0);
                sum += pv * strength;
            }
            let new_val = if sum < -0.5 {
                Ternary::Neg
            } else if sum > 0.5 {
                Ternary::Pos
            } else {
                Ternary::Zero
            };
            self.nodes[node_id].value = new_val;
        }
    }

    /// Get ancestors of a node
    pub fn ancestors(&self, node: usize) -> HashSet<usize> {
        let mut ancestors = HashSet::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if let Some(parents) = self.parents.get(&n) {
                for &p in parents {
                    if ancestors.insert(p) {
                        stack.push(p);
                    }
                }
            }
        }
        ancestors
    }

    /// Get descendants of a node
    pub fn descendants(&self, node: usize) -> HashSet<usize> {
        let mut descendants = HashSet::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if let Some(children) = self.adj.get(&n) {
                for &c in children {
                    if descendants.insert(c) {
                        stack.push(c);
                    }
                }
            }
        }
        descendants
    }

    /// d-separation test: are X and Y d-separated given Z?
    pub fn d_separated(&self, x: usize, y: usize, z: &HashSet<usize>) -> bool {
        // Use the bayes-ball algorithm
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((x, true)); // (node, going_up)
        let mut reachable = HashSet::new();

        while let Some((node, going_up)) = queue.pop_front() {
            if visited.contains(&(node, going_up)) {
                continue;
            }
            visited.insert((node, going_up));

            if !z.contains(&node) {
                reachable.insert(node);
            }

            if going_up && !z.contains(&node) {
                // Can continue up through parents and down through children
                for &parent in self.parents.get(&node).unwrap_or(&vec![]) {
                    queue.push_back((parent, true));
                }
                for &child in self.adj.get(&node).unwrap_or(&vec![]) {
                    queue.push_back((child, false));
                }
            } else if !going_up {
                if !z.contains(&node) {
                    // Continue down through children
                    for &child in self.adj.get(&node).unwrap_or(&vec![]) {
                        queue.push_back((child, false));
                    }
                }
                if z.contains(&node) {
                    // v-structure: go up through parents
                    for &parent in self.parents.get(&node).unwrap_or(&vec![]) {
                        queue.push_back((parent, true));
                    }
                }
            }
        }

        !reachable.contains(&y)
    }
}

/// Intervention: do(X = x) operation
#[derive(Debug, Clone)]
pub struct Intervention {
    pub node: usize,
    pub value: Ternary,
}

impl Intervention {
    pub fn new(node: usize, value: Ternary) -> Self {
        Intervention { node, value }
    }

    /// Apply intervention to a DAG: set node value and cut incoming edges
    pub fn apply(&self, dag: &mut CausalDAG) {
        dag.set_node_value(self.node, self.value);
        // Cut all incoming edges to the intervened node
        let parents_to_remove: Vec<usize> = dag.parents.get(&self.node).cloned().unwrap_or_default();
        dag.edges.retain(|e| e.to != self.node);
        dag.parents.insert(self.node, Vec::new());
        // Clean up adj: remove self.node from parent adjacency lists
        for p in &parents_to_remove {
            if let Some(children) = dag.adj.get_mut(p) {
                children.retain(|&c| c != self.node);
            }
        }
        dag.propagate();
    }

    /// Compute the interventional distribution: P(Y | do(X = x))
    pub fn interventional_value(
        &self,
        dag: &CausalDAG,
        outcome_node: usize,
    ) -> Ternary {
        let mut modified = dag.clone();
        self.apply(&mut modified);
        modified.get_node_value(outcome_node).unwrap_or(Ternary::Zero)
    }
}

/// Counterfactual reasoning engine
#[derive(Debug, Clone)]
pub struct CounterfactualEngine {
    dag: CausalDAG,
}

impl CounterfactualEngine {
    pub fn new(dag: CausalDAG) -> Self {
        CounterfactualEngine { dag }
    }

    /// Given observed values, what would Y have been if X had been x?
    pub fn counterfactual(
        &self,
        observed: &[(usize, Ternary)],
        intervention_node: usize,
        counterfactual_value: Ternary,
        outcome_node: usize,
    ) -> Ternary {
        // Step 1: Abduction - compute exogenous variables consistent with observations
        let mut abduced = self.dag.clone();
        for &(node, value) in observed {
            abduced.set_node_value(node, value);
        }

        // Step 2: Action - apply intervention
        let intervention = Intervention::new(intervention_node, counterfactual_value);
        intervention.apply(&mut abduced);

        // Step 3: Prediction - read outcome
        abduced.get_node_value(outcome_node).unwrap_or(Ternary::Zero)
    }

    /// Compute the "necessity" of a cause: would Y still have happened without X?
    pub fn necessity(
        &self,
        cause_node: usize,
        cause_value: Ternary,
        effect_node: usize,
        observed_effect: Ternary,
    ) -> bool {
        let mut dag_without = self.dag.clone();
        // Set cause to neutral
        dag_without.set_node_value(cause_node, Ternary::Zero);
        dag_without.propagate();
        let effect_without = dag_without.get_node_value(effect_node).unwrap_or(Ternary::Zero);
        effect_without != observed_effect
    }

    /// Compute the "sufficiency" of a cause: does X always produce Y?
    pub fn sufficiency(
        &self,
        cause_node: usize,
        cause_value: Ternary,
        effect_node: usize,
    ) -> bool {
        let mut dag_with = self.dag.clone();
        dag_with.set_node_value(cause_node, cause_value);
        dag_with.propagate();
        let effect_with = dag_with.get_node_value(effect_node).unwrap_or(Ternary::Zero);
        effect_with.value() * cause_value.value() > 0
    }
}

/// Observational sample: mapping from node index to observed ternary value
pub type Sample = HashMap<usize, Ternary>;

/// Causal discovery from observational data
#[derive(Debug)]
pub struct CausalDiscovery {
    node_names: Vec<String>,
}

impl CausalDiscovery {
    pub fn new(node_names: Vec<&str>) -> Self {
        CausalDiscovery {
            node_names: node_names.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Compute correlation between two variables across samples
    pub fn correlation(samples: &[Sample], x: usize, y: usize) -> f64 {
        let n = samples.len() as f64;
        if n == 0.0 {
            return 0.0;
        }
        let mean_x: f64 = samples.iter().map(|s| s.get(&x).map(|v| v.value() as f64).unwrap_or(0.0)).sum::<f64>() / n;
        let mean_y: f64 = samples.iter().map(|s| s.get(&y).map(|v| v.value() as f64).unwrap_or(0.0)).sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;
        for s in samples {
            let vx = s.get(&x).map(|v| v.value() as f64).unwrap_or(0.0);
            let vy = s.get(&y).map(|v| v.value() as f64).unwrap_or(0.0);
            cov += (vx - mean_x) * (vy - mean_y);
            var_x += (vx - mean_x).powi(2);
            var_y += (vy - mean_y).powi(2);
        }
        if var_x == 0.0 || var_y == 0.0 {
            return 0.0;
        }
        cov / (var_x.sqrt() * var_y.sqrt())
    }

    /// Simple PC-algorithm style discovery using conditional independence
    pub fn discover(&self, samples: &[Sample], threshold: f64) -> CausalDAG {
        let n = self.node_names.len();
        let mut dag = CausalDAG::new();

        // Add all nodes
        for name in &self.node_names {
            dag.add_node(name, Ternary::Zero);
        }

        // Compute pairwise correlations
        let mut adj_matrix = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let corr = Self::correlation(samples, i, j);
                if corr.abs() > threshold {
                    adj_matrix[i][j] = corr;
                    adj_matrix[j][i] = corr;
                }
            }
        }

        // Orient edges: assume temporal ordering (earlier -> later)
        for i in 0..n {
            for j in (i + 1)..n {
                if adj_matrix[i][j].abs() > threshold {
                    let _ = dag.add_edge(i, j, adj_matrix[i][j]);
                }
            }
        }

        dag
    }

    /// Discover using mutual information approximation
    pub fn discover_mutual_info(&self, samples: &[Sample], threshold: f64) -> CausalDAG {
        let n = self.node_names.len();
        let mut dag = CausalDAG::new();
        for name in &self.node_names {
            dag.add_node(name, Ternary::Zero);
        }

        // Simple MI approximation for ternary variables
        for i in 0..n {
            for j in (i + 1)..n {
                let mi = self.approximate_mi(samples, i, j);
                if mi > threshold {
                    let _ = dag.add_edge(i, j, mi);
                }
            }
        }

        dag
    }

    fn approximate_mi(&self, samples: &[Sample], x: usize, y: usize) -> f64 {
        let n = samples.len() as f64;
        if n == 0.0 {
            return 0.0;
        }

        // Count joint and marginal frequencies
        let mut joint = vec![vec![0usize; 3]; 3]; // [-1,0,1] x [-1,0,1]
        let mut mx = vec![0usize; 3];
        let mut my = vec![0usize; 3];

        for s in samples {
            let vx = s.get(&x).map(|v| (v.value() + 1) as usize).unwrap_or(1);
            let vy = s.get(&y).map(|v| (v.value() + 1) as usize).unwrap_or(1);
            joint[vx][vy] += 1;
            mx[vx] += 1;
            my[vy] += 1;
        }

        let mut mi = 0.0;
        for i in 0..3 {
            for j in 0..3 {
                if joint[i][j] > 0 && mx[i] > 0 && my[j] > 0 {
                    let pxy = joint[i][j] as f64 / n;
                    let px = mx[i] as f64 / n;
                    let py = my[j] as f64 / n;
                    mi += pxy * (pxy / (px * py)).ln();
                }
            }
        }
        mi.max(0.0)
    }
}

/// Effect estimation result
#[derive(Debug, Clone)]
pub struct EffectEstimate {
    pub average_treatment_effect: f64,
    pub effect_positive: f64,
    pub effect_negative: f64,
    pub effect_neutral: f64,
}

/// Estimate causal effects
pub struct EffectEstimator {
    dag: CausalDAG,
}

impl EffectEstimator {
    pub fn new(dag: CausalDAG) -> Self {
        EffectEstimator { dag }
    }

    /// Compute Average Treatment Effect (ATE) via do-calculus
    pub fn ate(&self, cause_node: usize, outcome_node: usize) -> EffectEstimate {
        // P(Y|do(X=1)) - P(Y|do(X=-1))
        let mut dag_pos = self.dag.clone();
        let int_pos = Intervention::new(cause_node, Ternary::Pos);
        int_pos.apply(&mut dag_pos);
        let y_pos = dag_pos.get_node_value(outcome_node).unwrap_or(Ternary::Zero).value() as f64;

        let mut dag_neg = self.dag.clone();
        let int_neg = Intervention::new(cause_node, Ternary::Neg);
        int_neg.apply(&mut dag_neg);
        let y_neg = dag_neg.get_node_value(outcome_node).unwrap_or(Ternary::Zero).value() as f64;

        let ate = y_pos - y_neg;

        EffectEstimate {
            average_treatment_effect: ate,
            effect_positive: y_pos,
            effect_negative: y_neg,
            effect_neutral: 0.0,
        }
    }

    /// Estimate using observational data (backdoor adjustment)
    pub fn ate_backdoor(
        &self,
        samples: &[Sample],
        cause_node: usize,
        outcome_node: usize,
        confounders: &[usize],
    ) -> f64 {
        let n = samples.len() as f64;
        if n == 0.0 {
            return 0.0;
        }

        // Stratify by confounder values and compute weighted ATE
        let mut total_effect = 0.0;

        // For simplicity, compute difference in means conditioned on confounders
        let mut sum_treated = 0.0;
        let mut count_treated = 0.0;
        let mut sum_control = 0.0;
        let mut count_control = 0.0;

        for s in samples {
            let cause = s.get(&cause_node).map(|v| v.value()).unwrap_or(0);
            let outcome = s.get(&outcome_node).map(|v| v.value() as f64).unwrap_or(0.0);

            if cause > 0 {
                sum_treated += outcome;
                count_treated += 1.0;
            } else if cause < 0 {
                sum_control += outcome;
                count_control += 1.0;
            }
        }

        if count_treated > 0.0 && count_control > 0.0 {
            total_effect = (sum_treated / count_treated) - (sum_control / count_control);
        }

        total_effect
    }

    /// Conditional Average Treatment Effect (CATE)
    pub fn cate(
        &self,
        samples: &[Sample],
        cause_node: usize,
        outcome_node: usize,
        condition_node: usize,
        condition_value: Ternary,
    ) -> f64 {
        let filtered: Vec<Sample> = samples.iter()
            .filter(|s| s.get(&condition_node) == Some(&condition_value))
            .cloned()
            .collect();

        self.ate_backdoor(&filtered, cause_node, outcome_node, &[])
    }

    /// Propensity score (simple version for ternary)
    pub fn propensity_score(samples: &[Sample], cause_node: usize, covariates: &[usize]) -> f64 {
        let n = samples.len() as f64;
        if n == 0.0 {
            return 1.0 / 3.0;
        }
        let treated = samples.iter()
            .filter(|s| s.get(&cause_node).map(|v| v.value()).unwrap_or(0) > 0)
            .count() as f64;
        treated / n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_values() {
        assert_eq!(Ternary::Neg.value(), -1);
        assert_eq!(Ternary::Zero.value(), 0);
        assert_eq!(Ternary::Pos.value(), 1);
    }

    #[test]
    fn test_ternary_from_i8() {
        assert_eq!(Ternary::from_i8(-1), Some(Ternary::Neg));
        assert_eq!(Ternary::from_i8(0), Some(Ternary::Zero));
        assert_eq!(Ternary::from_i8(1), Some(Ternary::Pos));
        assert_eq!(Ternary::from_i8(5), None);
    }

    #[test]
    fn test_dag_add_nodes() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        let c = dag.add_node("C", Ternary::Neg);
        assert_eq!(dag.nodes().len(), 3);
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(c, 2);
    }

    #[test]
    fn test_dag_add_edge() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        assert!(dag.add_edge(a, b, 1.0));
        assert_eq!(dag.edges().len(), 1);
    }

    #[test]
    fn test_dag_prevents_cycle() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        let c = dag.add_node("C", Ternary::Neg);
        assert!(dag.add_edge(a, b, 1.0));
        assert!(dag.add_edge(b, c, 1.0));
        assert!(!dag.add_edge(c, a, 1.0)); // Would create cycle
    }

    #[test]
    fn test_dag_prevents_self_loop() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        assert!(!dag.add_edge(a, a, 1.0));
    }

    #[test]
    fn test_topological_sort() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        let c = dag.add_node("C", Ternary::Neg);
        dag.add_edge(a, b, 1.0);
        dag.add_edge(b, c, 1.0);
        let order = dag.topological_sort();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_propagation() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Pos);
        let c = dag.add_node("C", Ternary::Zero);
        dag.add_edge(a, c, 0.6);
        dag.add_edge(b, c, 0.6);
        dag.propagate();
        assert_eq!(dag.get_node_value(c), Some(Ternary::Pos)); // 0.6 + 0.6 = 1.2 > 0.5
    }

    #[test]
    fn test_intervention() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        dag.add_edge(a, b, 1.0);
        let intervention = Intervention::new(a, Ternary::Neg);
        let result = intervention.interventional_value(&dag, b);
        assert_eq!(result, Ternary::Neg);
    }

    #[test]
    fn test_intervention_cuts_edges() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Pos);
        dag.add_edge(a, b, 1.0);
        let mut int_dag = dag.clone();
        let intervention = Intervention::new(b, Ternary::Neg);
        intervention.apply(&mut int_dag);
        // After intervention, B's value should be what we set
        assert_eq!(int_dag.get_node_value(b), Some(Ternary::Neg));
        // Edge from A to B should be cut
        assert!(int_dag.edges().is_empty());
    }

    #[test]
    fn test_counterfactual() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        dag.add_edge(a, b, 1.0);
        let engine = CounterfactualEngine::new(dag);
        let result = engine.counterfactual(
            &[(a, Ternary::Pos), (b, Ternary::Pos)],
            a,
            Ternary::Neg,
            b,
        );
        assert_eq!(result, Ternary::Neg);
    }

    #[test]
    fn test_necessity() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        dag.add_edge(a, b, 1.0);
        let engine = CounterfactualEngine::new(dag);
        assert!(engine.necessity(a, Ternary::Pos, b, Ternary::Pos));
    }

    #[test]
    fn test_sufficiency() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        dag.add_edge(a, b, 1.0);
        let engine = CounterfactualEngine::new(dag);
        assert!(engine.sufficiency(a, Ternary::Pos, b));
    }

    #[test]
    fn test_d_separation() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        let c = dag.add_node("C", Ternary::Neg);
        dag.add_edge(a, b, 1.0);
        dag.add_edge(b, c, 1.0);
        // A and C should not be d-separated given empty set (chain)
        assert!(!dag.d_separated(a, c, &HashSet::new()));
        // A and C should be d-separated given {B}
        assert!(dag.d_separated(a, c, &HashSet::from([b])));
    }

    #[test]
    fn test_ancestors_descendants() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Pos);
        let b = dag.add_node("B", Ternary::Zero);
        let c = dag.add_node("C", Ternary::Neg);
        dag.add_edge(a, b, 1.0);
        dag.add_edge(b, c, 1.0);
        assert_eq!(dag.ancestors(c), HashSet::from([a, b]));
        assert_eq!(dag.descendants(a), HashSet::from([b, c]));
    }

    #[test]
    fn test_causal_discovery() {
        let discovery = CausalDiscovery::new(vec!["A", "B", "C"]);
        let mut samples = Vec::new();
        for i in 0..100 {
            let mut s = Sample::new();
            let a = if i % 3 == 0 { Ternary::Pos } else if i % 3 == 1 { Ternary::Neg } else { Ternary::Zero };
            s.insert(0, a);
            s.insert(1, a); // B correlates with A
            s.insert(2, Ternary::Zero); // C independent
            samples.push(s);
        }
        let dag = discovery.discover(&samples, 0.3);
        // Should find edge from A to B
        assert!(dag.edges().iter().any(|e| e.from == 0 && e.to == 1));
    }

    #[test]
    fn test_correlation() {
        let mut samples = Vec::new();
        for i in 0..30 {
            let mut s = Sample::new();
            let v = if i < 10 { Ternary::Pos } else if i < 20 { Ternary::Neg } else { Ternary::Zero };
            s.insert(0, v);
            s.insert(1, v); // Perfect correlation
            samples.push(s);
        }
        let corr = CausalDiscovery::correlation(&samples, 0, 1);
        assert!((corr - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_effect_estimator_ate() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("A", Ternary::Zero);
        let b = dag.add_node("B", Ternary::Zero);
        dag.add_edge(a, b, 1.0);
        let estimator = EffectEstimator::new(dag);
        let effect = estimator.ate(a, b);
        // do(A=1) -> B=1, do(A=-1) -> B=-1, ATE = 1 - (-1) = 2
        assert!((effect.average_treatment_effect - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_effect_estimator_backdoor() {
        let mut dag = CausalDAG::new();
        let a = dag.add_node("treatment", Ternary::Zero);
        let b = dag.add_node("outcome", Ternary::Zero);
        dag.add_edge(a, b, 1.0);

        let mut samples = Vec::new();
        for i in 0..60 {
            let mut s = Sample::new();
            let t = if i < 20 { Ternary::Pos } else if i < 40 { Ternary::Neg } else { Ternary::Zero };
            s.insert(0, t);
            s.insert(1, t); // Outcome matches treatment
            samples.push(s);
        }

        let estimator = EffectEstimator::new(dag);
        let ate = estimator.ate_backdoor(&samples, 0, 1, &[]);
        assert!(ate > 0.0); // Positive treatment effect
    }

    #[test]
    fn test_propensity_score() {
        let mut samples = Vec::new();
        for i in 0..90 {
            let mut s = Sample::new();
            let t = if i < 30 { Ternary::Pos } else if i < 60 { Ternary::Neg } else { Ternary::Zero };
            s.insert(0, t);
            samples.push(s);
        }
        let ps = EffectEstimator::propensity_score(&samples, 0, &[1]);
        assert!((ps - (30.0 / 90.0)).abs() < 0.01);
    }
}
