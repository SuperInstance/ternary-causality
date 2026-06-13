# Ternary Causality

**Ternary Causality** provides causal inference for ternary strategy systems — featuring CausalDAG construction, do-calculus interventions, counterfactual reasoning, causal discovery from observational data, and treatment effect estimation.

## Why It Matters

Correlation doesn't imply causation. When fleet agents change strategies, we need to know: did the strategy change cause the outcome, or was a confounder responsible? Ternary Causality provides Pearl's do-calculus adapted for ternary variables {-1, 0, +1}, enabling intervention analysis ("what if we forced strategy +1?"), counterfactual reasoning ("what would have happened with strategy -1?"), and effect estimation (ATE, CATE). This moves fleet optimization from observational to causal.

## How It Works

### Causal DAG

A directed acyclic graph where nodes are ternary variables and edges represent causal relationships:

```
A → B → C
     ↗
A → D

CausalDAG {
    nodes: HashSet<Variable>,
    edges: HashSet<(Variable, Variable)>,  // (cause → effect)
}
```

Topological sort: **O(V + E)** (Kahn's algorithm). Cycle detection: **O(V + E)** (DFS).

### Do-Calculus Intervention

`do(X = x)` forces X to value x, severing incoming edges:

```
P(Y | do(X=x)) = Σ_z P(Y | X=x, Z=z) · P(Z=z)
```

This differs from observational P(Y | X=x) because it removes confounding paths through X's parents.

Implementation:
1. Find parents of X: **O(V)**
2. Backdoor adjustment over parent set Z: **O(|Z| · 3^|Z|)** (ternary enumeration)

### Counterfactual Engine

"Given we observed X=x, Y=y, what would Y have been if X=x'?"

Three-step procedure (Pearl):
1. **Abduction**: Update P(U | evidence) — posterior over exogenous variables
2. **Action**: Set X = x' in the structural model
3. **Prediction**: Compute P(Y | modified model)

Counterfactual: **O(3^|U|)** in general, **O(|U|)** for linear models.

### Causal Discovery

Learn causal structure from observational data using constraint-based (PC algorithm) or score-based methods:

PC Algorithm:
1. Start with complete graph
2. Remove edge X-Y if conditional independence test passes
3. Orient edges using collider detection (v-structures)

CI test for ternary: χ² test on 3×3 contingency table. Cost: **O(N)** per test (N = samples). Total: **O(V² · N)** worst case.

### Effect Estimation

```
ATE (Average Treatment Effect) = E[Y | do(X=+1)] - E[Y | do(X=-1)]
CATE (Conditional ATE) = ATE conditioned on subgroups
```

ATE estimation via backdoor adjustment: **O(|Z| · 3^|Z| · N)** for adjustment set Z and N samples.

## Quick Start

```rust
use ternary_causality::{CausalDAG, Ternary, Intervention};

let mut dag = CausalDAG::new();
dag.add_edge("strategy", "performance");
dag.add_edge("resources", "performance");

let ate = dag.estimate_ate("strategy", "performance");
println!("ATE of strategy on performance: {:.3}", ate);
```

## API

| Type | Description |
|------|-------------|
| `CausalDAG` | Directed acyclic graph of causal relationships |
| `Intervention` | do-calculus intervention specification |
| `CounterfactualEngine` | "What if?" reasoning |
| `CausalDiscovery` | Learn structure from data (PC algorithm) |
| `EffectEstimator` | ATE, CATE, and heterogeneous treatment effects |
| `Ternary` | Neg (-1), Zero (0), Pos (+1) |

## Architecture Notes

Ternary Causality provides causal reasoning for fleet strategy evaluation in SuperInstance. In γ + η = C, the do-calculus answers "what causes γ (growth)?" while counterfactuals reveal η (what we avoid by choosing γ over alternatives). The conservation law γ + η = C is itself a causal claim: the sum is invariant under strategy perturbations that conserve total effort.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for causal inference architecture.

## References

1. Pearl, J. (2009). *Causality: Models, Reasoning, and Inference*, 2nd ed. Cambridge University Press.
2. Spirtes, P. et al. (2000). *Causation, Prediction, and Search*, 2nd ed. MIT Press.
3. Hernán, M. A. & Robins, J. M. (2020). *Causal Inference: What If*. Chapman & Hall/CRC.

## License

MIT
