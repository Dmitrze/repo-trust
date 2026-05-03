# Boris Cherny — Claude Workflow Playbook

> **Source.** Distilled from Boris Cherny's working method with Claude Code. The 11-step cycle is what every non-trivial change in this repo follows.
>
> **How to use this document.** This is a working method, not a checklist to perform. Internalize the eleven steps; apply them in order on the first pass; deviate only when a specific principle clearly does not apply to a specific task.

---

## 1. Start with the goal, not the code

Formulate why the project (or feature) exists in one or two sentences in plain language. State who it's for, what pain it removes, and what specific signal will tell us "this is working" — a metric, an event, a concrete result. Only after that, think about features, architecture, tools.

**How to apply.**
- Write 1-2 sentences: "I am building X for Y so they can Z."
- Add an explicit success criterion: "Pilot is done when I have 3 paying clients" / "feature is done when the bot completes the end-to-end scenario 10 times" / etc.

**Why this matters.** It gives Claude Code the right vector. It does not drift into abstract engineering; it optimizes for the concrete outcome.

---

## 2. Turn the idea into one concrete scenario

Instead of "build a platform," describe one concrete working scenario step by step: what the user does and what should happen in the system. Focus on the happy path — the ideal flow without errors, from start to finish.

**How to apply.**
- Describe one scenario: who the user is, the starting point, the sequence of actions, what a successful end state looks like.

**Why this matters.** Models design code more reliably around concrete events than around vague "capabilities." For you, this scenario is the immediate basis for E2E tests and demos.

---

## 3. Decompose into 3-7 modules

Divide the system into a small number of meaningful modules — not 20, but 3 to 7. Each module owns one clear concern: "API backend," "UI," "integration with external service," "data processing pipeline," etc.

**How to apply.**
- List modules at the architectural-block level, not at the function level.
- For each module, write 1-2 lines: responsibility, inputs, outputs.

**Why this matters.** Claude Code can then work modularly: extend, refactor, and test each block separately, instead of mashing everything into one file.

---

## 4. Hard constraints and an explicit stack

Give the model constraints. For example: language (TypeScript / Python / Go); framework (Next.js, FastAPI, etc.); infrastructure (Docker, AWS Lambda, Supabase). Also constrain the iteration length: "Build the first version only for this scenario — no auth, no optimization."

**How to apply.**
- Explicitly list the stack and the bans: "Use only X and Y. Do not use Z."
- Define the iteration boundary: what is *not* in this version.

**Why this matters.** By default models "accelerate" — they add everything plausible. Constraints sharply increase relevance and speed.

---

## 5. Skeleton first, flesh later

The skeleton comes before any depth: files, directories, base entry points, API stubs, then details. Ask the AI to create a minimal working scaffold that runs: one command path, one window, one endpoint.

**How to apply.**
- In the first request to Claude, do not ask for "the full product." Ask for: project structure (folders, files); minimal code that compiles / runs and executes the simplest happy path.

**Why this matters.** You get something runnable and demoable very quickly. After that, iterations work on a live project, not against an abstraction.

---

## 6. Small iterations and fast cycles

Do not send huge requests. Work in short cycles:

1. Formulate a small goal.
2. Ask the model.
3. Run the code; see the result.
4. Give very concrete feedback (logs, error, diff).

Each cycle improves either functionality, DX (structure, readability), or UX.

**How to apply.**
- Break work into steps that can be completed in 5 to 20 minutes.
- After each step, run the code and attach to the next request: the error (stack trace), or the diff / fragment that needs fixing.

**Why this matters.** Models work great as an additional developer in a loop — not as a generator of huge code blocks in a vacuum.

---

## 7. Code-as-conversation: maximize context

Feed the model the project context: key files, configurations, important code fragments — not abstract descriptions. Write requests in pair-programming style, not magic-genie style: "Here is the current code for X. Here is error Y. Here is goal Z. Help me fix it."

**How to apply.**
- In each request: a short reminder of the project goal; the current code fragment / structure; a concrete question or task.
- Use code paste-ins instead of word descriptions where possible.

**Why this matters.** Claude Code makes decisions based on the code it actually sees. Show a real project and it acts much more sensibly.

---

## 8. Clear quality standards

Set quality criteria: tests, logging, readability, simple APIs. Explicitly say: "Write code that is easy for other developers to maintain. Simplicity first, optimization later."

**How to apply.**
- Define a minimum Definition of Done for code: there are basic tests for the key scenario; the code is split into small functions; names and interfaces are clear without comments.

**Why this matters.** Without explicit criteria, the model often writes "demo code" that's hard to evolve. With criteria, it optimizes directly for them.

---

## 9. Documentation and interface for the future you

Think about the future-you and others. Every project starts with a short README and / or dev instructions. Ask the model: "Generate a README and comments that will explain the project structure to a new developer."

**How to apply.**
- After the skeleton is ready, ask Claude to: write a README with goal, stack, how to run, basic scenario; a short architecture description (1-2 paragraphs).

**Why this matters.** This locks the architecture in words and serves as a contract between you and the model on later iterations.

---

## 10. Conscious work with model boundaries

The model can hallucinate libraries, methods, APIs. Ask it to verify compatibility with the stack and the version. Use the model for code generation, refactoring, explanations — but never trust it blindly. There is always verification by running and testing.

**How to apply.**
- Always specify the language / framework version.
- Ask for the rationale behind the chosen approach. Push back if it doesn't fit.
- Plan at least one "check + fix" cycle for every major generation.

**Why this matters.** Instead of fighting errors after the fact, you build the process around the assumption that errors will occur — and use the model as a companion, not an oracle.

---

## 11. Continuous strategy recalibration

As the project progresses, update the goal, scenarios, and decomposition if real constraints differ from the initial ones. Periodically reformulate the "main request" to the model with the current project state in mind.

**How to apply.**
- Every 1 to 2 days, or after each major feature, do a checkpoint prompt: describe what already exists; what is broken / slow; what you want to achieve in the next 2 to 3 iterations.
- Ask Claude Code to propose an updated plan.

**Why this matters.** Projects rarely follow the initial plan. Regular recalibration lets you use the model as a strategist, not just a code generator.

---

## How this connects to the rest of the stack

- Step 1 (goal) -> ends up in `/specs/<feature>.md` `## 1. Goal` section.
- Step 2 (scenario) -> ends up in `/tests/scenarios/<feature>.md` happy-path block.
- Step 3 (3-7 modules) -> the architecture sketch in the spec.
- Step 4 (constraints) -> the boundaries section in the spec, plus `REQUIREMENTS.md` for the project-wide stack.
- Step 5 (skeleton first) -> the Implementer's first commit.
- Steps 6-7 (small iterations, code-as-conversation) -> the Implementer's working rhythm.
- Step 8 (DoD) -> `CLAUDE.md` section 13.
- Step 9 (docs) -> the `Documenter` role (from the multi-agent template) or the Implementer's DoD.
- Step 10 (hallucinations) -> the Verifier role.
- Step 11 (recalibration) -> the weekly checkpoint, which lands in MemPalace `sessions` room.

In short: Boris's cycle is the unit-of-work pattern. The multi-agent template (separate doc) is the *who-runs-which-step* pattern. Both are required.
