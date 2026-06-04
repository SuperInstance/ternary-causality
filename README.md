# ternary-causality

Causal inference for ternary strategy systems — build causal DAGs, perform interventions (do-calculus), reason counterfactually, discover causal structure from data, and estimate treatment effects.

## Why This Exists

Many real-world systems have ternary outcomes: market direction (up/neutral/down), sentiment (positive/neutral/negative), or treatment response (improved/unchanged/worsened). Standard causal inference libraries assume continuous or binary variables. This crate provides a complete causal reasoning toolkit designed for the three-valued domain, from structural causal models to counterfactual analysis.

## Core Concepts

- **CausalDAG** — Directed acyclic graph for ternary causal models with cycle prevention, topological sort, and value propagation
- **Intervention** — Do-calculus `do(X = x)` operations that cut incoming edges and propagate effects
- **CounterfactualEngine** — "What would have happened if..." reasoning via abduction-action-prediction
- **CausalDiscovery** — Learn causal structure from observational data using correlation or mutual information
- **EffectEstimator** — Compute Average Treatment Effects (ATE), Conditional ATE (CATE), and propensity scores

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-causality = "0.1"
```

```rust
use ternary_causality::*;
use std::collections::{HashSet, HashMap};

// Build a causal DAG: Treatment → Outcome, confounded by Health
let mut dag = CausalDAG::new();
let health = dag.add_node("health", Ternary::Pos);
let treatment = dag.add_node("treatment", Ternary::Zero);
let outcome = dag.add_node("outcome", Ternary::Zero);

dag.add_edge(health, treatment, 0.4);   // health influences treatment
dag.add_edge(health, outcome, 0.6);     // health influences outcome
dag.add_edge(treatment, outcome, 0.8);  // treatment influences outcome

// Propagate values through the DAG
dag.propagate();
println!("Outcome: {:?}", dag.get_node_value(outcome));

// Intervention: do(treatment = Pos)
let intervention = Intervention::new(treatment, Ternary::Pos);
let effect = intervention.interventional_value(&dag, outcome);
println!("P(outcome | do(treatment=+1)): {:?}", effect);

// Counterfactual: what would outcome have been if treatment had been negative?
let engine = CounterfactualEngine::new(dag.clone());
let counterfactual = engine.counterfactual(
    &[(treatment, Ternary::Pos), (outcome, Ternary::Pos)],
    treatment,
    Ternary::Neg,
    outcome,
);
println!("Counterfactual outcome: {:?}", counterfactual);

// Causal necessity and sufficiency
println!("Necessary: {}", engine.necessity(treatment, Ternary::Pos, outcome, Ternary::Pos));
println!("Sufficient: {}", engine.sufficiency(treatment, Ternary::Pos, outcome));

// d-separation test
let separated = dag.d_separated(health, outcome, &HashSet::from([treatment]));
println!("d-separated(health, outcome | treatment): {}", separated);

// Estimate ATE
let estimator = EffectEstimator::new(dag);
let effect = estimator.ate(treatment, outcome);
println!("ATE: {:.4}", effect.average_treatment_effect);

// Discover causal structure from data
let discovery = CausalDiscovery::new(vec!["X", "Y", "Z"]);
let mut samples = Vec::new();
for i in 0..100 {
    let mut s = HashMap::new();
    let x = if i % 3 == 0 { Ternary::Pos } else { Ternary::Neg };
    s.insert(0, x);
    s.insert(1, x);  // Y correlates with X
    s.insert(2, Ternary::Zero);  // Z independent
    samples.push(s);
}
let learned_dag = discovery.discover(&samples, 0.3);
println!("Discovered {} edges", learned_dag.edges().len());
```

## API Overview

| Type / Function | Description |
|---|---|
| `Ternary` | Enum: `Neg`, `Zero`, `Pos` with `value()` → i8 |
| `CausalDAG` | DAG with `add_node`, `add_edge`, `propagate`, `topological_sort`, `d_separated` |
| `Intervention` | Do-calculus with `apply`, `interventional_value` |
| `CounterfactualEngine` | `counterfactual`, `necessity`, `sufficiency` |
| `CausalDiscovery` | `discover` (correlation), `discover_mutual_info` (MI), `correlation` |
| `EffectEstimator` | `ate`, `ate_backdoor`, `cate`, `propensity_score` |
| `EffectEstimate` | ATE result with positive/negative/neutral components |

## How It Works

**Causal DAG**: Nodes hold ternary values. Edges carry strength weights. Value propagation computes each node's value as the weighted sum of its parents' values, thresholded to ternary. Cycle detection via BFS prevents invalid graph structures.

**Interventions**: `do(X = x)` sets the node value and removes all incoming edges (mutilated graph), then re-propagates. This implements the causal (not observational) "do" operator from Pearl's do-calculus.

**Counterfactuals**: Three-step procedure: (1) Abduction — fix exogenous variables to match observations, (2) Action — apply the counterfactual intervention, (3) Prediction — read the outcome. Necessity asks "would the effect disappear without the cause?" Sufficiency asks "does the cause always produce the effect?"

**Causal Discovery**: PC-algorithm style approach — compute pairwise correlations (or mutual information) between variables, establish edges for statistically dependent pairs, orient edges using temporal ordering.

**Effect Estimation**: ATE is computed as `E[Y | do(X=+1)] − E[Y | do(X=−1)]`. The backdoor adjustment stratifies by confounders to deconfound the estimate. CATE conditions on a specific subpopulation.

## Use Cases

1. **Clinical trial analysis** — Estimate treatment effects for ternary outcomes (improved/unchanged/worsened) with confounder adjustment
2. **Market causal analysis** — Determine whether news sentiment causes price movement vs. mere correlation
3. **A/B testing with ternary metrics** — Reason about causality when outcomes are positive/neutral/negative
4. **Policy evaluation** — Counterfactual reasoning about what would have happened under alternative ternary policy decisions

## Ecosystem

Part of the **SuperInstance** ternary computing crate family:

- `ternary-compression-v2` — Multi-algorithm ternary compression
- `ternary-hash` — Hashing and fingerprinting for ternary data
- `ternary-pca` — Principal component analysis on ternary values
- `ternary-ga` — Genetic algorithms with ternary genomes
- `ternary-matrix` — Compact ternary matrix operations
- `ternary-reservoir` — Echo state networks with ternary nodes
- `ternary-evolution-advanced` — Advanced evolutionary optimization
- `ternary-geometry` — Geometric algorithms in ternary space
- `ternary-consensus` — Distributed consensus for ternary agents

## License

MIT
