# Future Integration: ternary-causality

## Current State
Provides causal inference for ternary systems: `CausalDAG` for directed acyclic causal relationships, `Intervention` for do-calculus style "what if" reasoning, `CounterfactualEngine` for "what would have happened" analysis, `CausalDiscovery` for learning causal structure from observational data, and `EffectEstimator` for computing ATE/CATE causal effects.

## Integration Opportunities

### With ternary-cell (Event Chain Tracing)
A cell grid's tick cycle produces a cascade of events: cell A's surprise triggers cell B's perception, which affects cell C's prediction. ternary-causality's `CausalDAG` traces these chains — edges connect cells where one's output directly caused another's state change. `CausalDiscovery` learns the grid's causal structure from observed tick histories, revealing which cells are causal bottlenecks (many downstream effects) vs. isolated (few connections).

### With ternary-replay (Counterfactual Replay)
ternary-replay records experiment histories. ternary-causality's `CounterfactualEngine` uses those recordings for "what if" analysis: given the recorded history, apply an `Intervention` at step N (change a cell's signal), and replay forward to see the counterfactual outcome. This enables debugging: "if cell 42 hadn't fired at tick 100, would the grid have converged differently?"

### With ternary-diff (Causal Diffs)
ternary-diff tracks state changes between ticks. ternary-causality annotates those changes with causal labels. A "causal diff" doesn't just show *what* changed but *why* — each diff hunk is tagged with the upstream cause. When two rooms exchange diffs via ternary-protocol, causal tags help the receiver understand the change's provenance and decide whether to accept or reject it.

## Potential in Mature Systems
In room-as-codespace, rooms interact through message passing. ternary-causality builds a causal graph across rooms: Room A's output caused Room B's state change, which caused Room C's anomaly. When a room produces an error, PLATO traces the causal chain backward through the DAG to find the root cause room. `EffectEstimator` quantifies how much each upstream room contributed to the error — essential for distributed debugging across Codespaces.

## Cross-Pollination Ideas
- **ternary-thermodynamics**: Causal inference on thermodynamic quantities — does entropy increase cause phase transitions, or vice versa? The causal DAG reveals the direction.
- **ternary-games**: Causal game theory — in a Nash equilibrium, which player's move was the causal driver? `CausalDAG` on game trees reveals causal structure in strategic interaction.
- **ternary-federated**: Federated causal discovery — learn causal structure across distributed rooms without sharing raw data, using only strategy summaries.

## Dependencies for Next Steps
- Define `CellCausalDAG` type mapping cell IDs to causal nodes
- Implement causal discovery from ternary-replay recordings
- Add causal annotation to ternary-diff hunks
- Build counterfactual replay engine for distributed debugging
