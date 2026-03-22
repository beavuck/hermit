## Environment

When a tool, command, or workflow behaves unexpectedly (sandbox restrictions, hooks rewriting output, missing binaries),
stop immediately and explain what's happening to the user — never attempt workarounds or try to bypass the environment.

## Behavior

Banish the word "perfect" from your vocab. Don't make a show of being confident -- the user values truth over
convenience.

Don't make a show of being skeptical either, just act with a critical mindset.

When in doubt, ask for clarification rather than making assumptions.

Never ask the user to share secrets, tokens, passwords, or credentials — instead, show the command with a placeholder
(e.g. `<token>`) and let them fill it in themselves.

## Critical Rules - Get Approval First

Before taking these actions, STOP and explain the situation to the user, then let them decide:

- **Deleting any test code**
    - When a test fails or seems problematic:
        - Explain the root cause
        - List some options (fix the test, fix the implementation, restructure, etc.)
        - Use AskUserQuestion to let the user choose
    - Example: "This test fails because of X. Options: (1) Remove test (2) Fix by doing Y (3) Change implementation to
      Z. Which would you prefer?"

- **Suppressing any warnings**
    - Explain what the warning means and why it's appearing
    - Let the user decide whether to suppress it

- **Making architectural decisions**
    - Choosing between different implementation approaches (e.g., StateFlow vs LiveData)
    - Changing public APIs / endpoints
    - Large refactorings not explicitly requested
    - This includes small decisions scoped within a feature (e.g., which module to put new code in, what the public API
      of a new method looks like) — present options and ask before writing any code.

- **Deleting any existing code** (except when replacing with new implementation)
    - Removing unused functions, classes, or files
    - Let the user decide if something is truly unused

- **Debugging loops**
    - After two failed attempts at the same fix, stop and explain the situation to the user before trying again — don't
      escalate unilaterally; the root cause is likely in a different layer than where you've been looking

## Pre-Action Checklist

Before using Edit or Write tools, verify:

- [ ] Am I deleting test code? → Ask user first
- [ ] Am I suppressing a warning? → Ask user first
- [ ] Am I making an architectural choice, even a small one (which module, what method signature)? → Ask user first
- [ ] Am I about to delete code I didn't write in this session? → Explain and ask

## Working in small shippable units

When given a specific task, do only that task. Do not:

- Implement additional related methods
- Try to complete the entire feature
- Make assumptions about what else needs to be done

Stop and ask for feedback at every natural checkpoint — after reading code, after writing tests, after each small
implementation step. Don't chain multiple steps together without a confirmation in between.

Let the user test each small unit before moving to the next. This allows:

- Catching issues early
- User to make adjustments on the fly
- Faster feedback cycles
- Less wasted effort if the direction changes

## Coding style and practices

- Avoid boolean parameters — they signal a function has more than one responsibility; offer the user a few options
  instead, and let the user give an open answer in case none of your options are chosen.
- Practice SOLID
    - S: Single Responsibility Principle (each class, each method, should have one reason to change)
    - O: Open/Closed Principle (classes should be open for extension but closed for modification)
    - L: Liskov Substitution Principle (subtypes must be substitutable for their base types)
    - I: Interface Segregation Principle (prefer many specific interfaces over a single general one)
    - D: Dependency Inversion Principle (depend on abstractions, not on concretions)
- Do not use comments. Except if it's exceptionally useful to explain _why_ a thing is done. Never
  use comments to explain _what_ is done -- the code should read clearly on its own.
- Don't repeat yourself. If two bits of code look alike but are bound to evolve in different ways,
  fair enough, but apart from that, duplicating code or logic should be banned.
- Don't hardcode magic values. If a value has a specific meaning, it should be defined as a constant with a descriptive
  name.
- Practice TDD: Write failing tests first, then stop and let the user confirm they fail before writing any
  implementation. Then write code to make them pass. But don't test for
  specific behavior within a method -- test for the observable effects of that behavior. Input in,
  output out.
    - Write unit tests for all core logic. Use Integration tests for cross-cutting concerns. API tests via
      Bruno / bru can also be useful for that.
    - When writing tests for functions or types that don't exist yet, add minimal stubs (e.g. `todo!()` bodies,
      empty structs, placeholder fields) so the project compiles and the tests can run and fail at runtime.
- Use meaningful names. Choose clear and descriptive names for variables, functions, classes, and
  modules.
- Phrase `expect`, `assert`, and error messages in the form "X should Y" — it describes the happy
  path and doubles as a readable failure message (e.g. `"spec file should be readable"`).
- Perform minimal changes necessary to implement features or fix bugs. Avoid large refactorings
  unless asked for.
- Never suppress any warnings -- let a human do so if they deem it necessary.
- Ensure you use no deprecated methods, APIs, or libraries.
- Favor immutability.
- Favor composition over inheritance.
- Prefer iterative style to recursive style; only use recursion when you can mathematically prove it is safe (bounded
  depth, no stack overflow risk) and faster.
- Follow Rust conventions and best practices for code structure, project layout, error handling, and performance.
- When a fix requires increasing complexity, stop and look for an approach that removes the dependency instead of
  working around it.
- Use contract functions. A contract function only invokes other functions, to execute some important, high level
  capability. This means the code must be broken out into enough functions that the contract function can be easily read
  by someone wanting to understand the main steps in executing this functionality.