## Behavior

Banish the word "perfect" from your vocab. Don't make a show of being confident -- the user values truth over
convenience.

Don't make a show of being skeptical either, just act with a critical mindset.

When in doubt, ask for clarification rather than making assumptions.

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
- [ ] Am I making an architectural choice? → Ask user first
- [ ] Am I about to delete code I didn't write in this session? → Explain and ask

## Working in small shippable units

When given a specific task, do only that task. Do not:

- Implement additional related methods
- Try to complete the entire feature
- Make assumptions about what else needs to be done

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
- Practice TDD: Write failing tests first. Then write code to make them pass. But don't test for
  specific behavior within a method -- test for the observable effects of that behavior. Input in,
  output out.
    - Write unit tests for all core logic. Use Integration tests for cross-cutting concerns. API tests via
      Bruno / bru can also be useful for that.
- Use meaningful names. Choose clear and descriptive names for variables, functions, classes, and
  modules.
- Perform minimal changes necessary to implement features or fix bugs. Avoid large refactorings
  unless asked for.
- Never suppress any warnings -- let a human do so if they deem it necessary.
- Ensure you use no deprecated methods, APIs, or libraries.
- Favor immutability.
- Favor composition over inheritance.
- Follow Rust conventions and best practices for code structure, project layout, error handling, and performance.
- When a fix requires increasing complexity, stop and look for an approach that removes the dependency instead of
  working around it.
