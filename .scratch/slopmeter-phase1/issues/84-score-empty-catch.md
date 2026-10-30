# Score an empty or swallowing catch

Type: task
Status: open
Blocked by: 83

## Question

Build the F15 rule the way [Whether an empty or swallowing catch ships as F15](83-empty-catch-rule.md) decides it. Each adapter records a handler that swallows on the IR function, `rules::evaluate` emits one finding per function at the first such handler with the spec's gate weight, the explain page and goldens are added, and the sloppy corpora gain one swallowing handler per language so the validation tests fail on it.
