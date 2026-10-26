# What counts as a function, per language

Type: grilling
Status: open
Blocked by: 

## Question

Which syntactic units does slopmeter score as a function in each language, and which are excluded or folded into their parent? Python: module-level defs, methods, nested defs, lambdas, async defs, decorated functions (spec says decorators are excluded from CC — but is the decorator call itself scored?), properties. Rust: free fns, impl methods, trait default methods, closures (standalone vs inline callbacks), fns inside macros, `const fn`, generic fns, test fns. C++: free functions, member functions (in-class and out-of-line), constructors/destructors (crap4java excludes constructors), operators, lambdas, templates (per definition, not per instantiation), functions defined in headers. For each language: does a closure or lambda get its own CC and LLOC, or count toward its enclosing function? How is a function identified (the `Function.id` in the IR) stably across runs — qualified name plus signature, file plus line, or a hash?
