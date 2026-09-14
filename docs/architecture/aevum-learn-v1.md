# Aevum Learn v1 — Architecture

Status: **B-Learn design in progress**

Aevum Learn is the entry point into the Aevum compute infrastructure. It is
not a marketing site, a course catalog, or a certificate factory. It is the
path through which a person can move from curiosity to meaningful
contribution — starting from fundamentals and ending wherever they choose.

---

## 1. Purpose & Philosophy

### 1.1 Why Aevum Exists

Computing power has become one of the most important infrastructures of
modern civilization. Scientific research, drug discovery, climate modeling,
machine learning, cryptography, and simulation all depend on access to
compute.

Today, much of this compute is concentrated in a relatively small number of
centralized data centers operated by large providers. This concentration
creates three structural problems:

- **Access**: independent researchers and small teams may not have access to
  competitive compute at sustainable cost.
- **Geography**: capacity clusters in a limited number of regions, while
  demand is global.
- **Idle capacity**: significant amounts of compute can remain unused in
  homes, laboratories, universities, and businesses.

Aevum is an attempt to address this imbalance. Aevum aims to turn
underutilized computing resources — GPUs, CPUs, and other devices — into an
open, decentralized compute infrastructure.

Short positioning:

> **Aevum — decentralized compute infrastructure.**

Mission statement:

> **Aevum strives to turn the idle computing resources of humanity into
> open, global compute infrastructure.**

The word *strives* is deliberate. Aevum does not claim to already operate at
global scale. It defines a direction and a set of architectural commitments.

### 1.2 Why Learn Exists

The long-term constraint is not audience size. It is engineering capacity.

Aevum Learn is not a marketing site. It is not a course catalog. It is not a
certificate factory.

Aevum Learn is the **entry point into the Aevum infrastructure** — the place
where a person with no prior experience can begin, and where an experienced
engineer can go deeper into protocol design, distributed verification, or
cryptography.

Learn exists because a decentralized compute network cannot be built by a
small core team alone. It must be built by independent contributors, each
bringing their own competence, their own pace, and their own reasons.

Learn serves a single purpose:

> **To help people become capable of understanding, using, and building
> decentralized compute infrastructure — starting from fundamentals, and
> ending wherever they choose to go.**

---

### 1.3 Six Principles

Aevum is governed by six architectural and cultural principles. They are not
slogans. They describe how the project behaves and what it refuses to become.

**1. Open.**

The technology is developed in the open. Code, specifications, research
notes, and architectural decisions are public — so that independent people
can study, audit, critique, and improve the system.

**2. Decentralized.**

Aevum does not depend on a centrally owned compute facility or a single
organization as the foundation of the network. Decentralization applies not
only to compute nodes, but also to the ability to develop, operate, verify,
and build upon the network independently.

**3. Useful.**

Compute resources are directed toward work that has practical value. This
includes scientific computing, AI inference, fine-tuning, optimization,
rendering, simulation, cryptographic workloads, and other verifiable tasks.
Aevum does not burn energy on pointless hashing.

**4. Voluntary.**

Participation is voluntary. Nobody owes Aevum their time, their hardware, or
their attention. A contributor can make one commit, contribute for years,
step away, and return later. This is normal and accepted.

**5. Composable.**

Anyone — an individual, a researcher, a team, or a company — can build on
top of the Aevum protocol. Aevum defines primitives, not a closed platform.

**6. Real.**

Aevum is not a set of promises. It is working code, working nodes, real
workloads, and real results. Where results are not yet proven, the
documentation says so. Where claims cannot be measured, they are not made.

A note on blockchain:

> **Blockchain is coordination infrastructure, not the purpose of Aevum.**

Aevum uses blockchain — its L1 protocol and L2 execution environment — to
coordinate identity, presence, settlement, and programmable economics across
independent compute providers. The purpose is the compute network. The
blockchain is the mechanism that lets the network function without a central
authority.

### 1.4 The Bee

A bee does not build a giant factory. It moves between many places, gathers
small amounts of resource, and returns them to a shared system. Millions of
independent bees produce a result far larger than any single bee could
produce alone.

The bee represents the architectural idea behind Aevum.

Aevum does not need to own a massive data center. It uses many independent
devices. Each device contributes a portion of its compute. Tasks are
distributed across many nodes. The combined result is global compute
infrastructure built from many small contributions.

> **One GPU is a resource. Millions of independent GPUs become
> infrastructure.**

The bee is more than a mascot. It represents the architectural principle
behind Aevum: many independent contributions forming a system larger than
any individual contribution.

---

## 2. Audience

### 2.1 Who This Is For

Aevum Learn is designed for people who want to **build**, not just watch. It
is open to anyone who is willing to learn and contribute, regardless of
prior experience.

The audience includes, but is not limited to:

- **Students and beginners** with no prior programming experience who want a
  real path into systems engineering, cryptography, or distributed computing.
- **Developers** with experience in other languages who want to learn Rust,
  distributed systems, and protocol engineering on a real project.
- **Cryptographers and researchers** interested in post-quantum cryptography,
  consensus design, verification strategies, or protocol economics.
- **Researchers and scientists** who need distributed computational resources
  for reproducible workloads.
- **Compute developers and workload authors** who want to package,
  distribute, execute, and verify computational workloads across
  decentralized infrastructure.
- **Hardware enthusiasts and node operators** who want to run a compute node,
  contribute GPU or CPU capacity, or experiment with decentralized
  infrastructure.
- **Contributors of any kind** — documentation writers, testers, reviewers,
  translators, educators, designers — anyone whose work improves the
  ecosystem.
- **Anyone curious** who wants to understand how decentralized compute
  infrastructure actually works, even if they never write a line of code.

Learn does not require a specific background. It requires only a willingness
to learn.

### 2.2 Who This Is Not For

Aevum Learn is **not**:

- A place to earn tokens by completing lessons.
- A place that promises employment, income, or career advancement.
- A place where joining means becoming an "employee" of Aevum.
- A place that requires a specific number of hours, a schedule, or any
  long-term commitment.
- A place that issues certificates that substitute for real engineering
  competence.

Aevum does not promise any financial outcome. The economics of the Aevum
network and the purpose of Aevum Learn are separate. Learn exists to build
competence. What a person does with that competence is entirely their own
decision.

---

### 2.3 Participants, Not Employees

Aevum is not a company. It has no hiring process, no office, no mandatory
staff, and no central employer.

The people who build Aevum are **participants**, not employees. They
contribute because they choose to. They can contribute one pull request or
ten thousand. They can participate for a month or a decade. They can leave at
any time and return at any time.

The relationship is straightforward:

- Aevum offers an open protocol, documentation, and a community.
- Participants offer whatever contribution they choose.

Participation in Aevum is not an offer of employment, and becoming a
contributor does not create an employment relationship. Participation is
voluntary; contributing to the project does not by itself create an
employment, contractual, or financial obligation.

Levels such as *Contributor*, *Core Contributor*, or *Maintainer* describe
levels of **trust, competence, and responsibility within the open project** —
not job titles, not employment status, and not a management hierarchy.

> **Aevum does not invite people to work for Aevum. Aevum invites people to
> build alongside others who share the idea of open, decentralized compute
> infrastructure — if and when they choose to.**

### 2.4 Fundamentals First

Aevum Learn does not only teach Aevum.

Before a person can meaningfully contribute to decentralized compute
infrastructure, they need foundational knowledge:

```text
Computer Science
        ↓
Linux and Networking
        ↓
Rust
        ↓
Cryptography
        ↓
Distributed Systems
        ↓
Blockchain Fundamentals
        ↓
Aevum Architecture
```

The sequence is a recommended foundation, not a mandatory gate.

The goal is not to produce people who can operate Aevum's tools. The goal is
to produce engineers who can understand and build decentralized
infrastructure — whether that infrastructure is Aevum's or something else
entirely.

This is what makes the difference between a user and an engineer.

Practice over certificates.
Real engineering over marketing.
Real contribution over follower counts.

---

## 3. Learning Paths

### 3.0 Overview

Learning paths are **routes**, not a curriculum. A route tells you where you
can go, not every step you must take. Paths overlap, branch, and can be
entered from different starting points. No path requires you to complete any
other path first.

You do not need permission to start. You do not need a certificate to move
to the next path. You choose the route based on where you are now and where
you want to go.

```text
                       LEARNING PATHS

       Beginner ─────── Rust / Systems
           │                  │
           ├──── Cryptography ┤
           │                  │
           ├──── Protocol / Blockchain
           │                  │
           └──── Compute / GPU

                  ↓
           Contributor Path
                  ↓
   Trust / Competence / Responsibility
                  ↓
      Core Contributor / Maintainer
```

These connections are examples, not prerequisites. You can enter any
path from any starting point, leave any path at any time, and combine
paths however you choose.

Each path is a starting point, not a sequence requirement. A path can be
left at any time and returned to later, or replaced by another path if
interests change.

### 3.1 Beginner Path

**For:** People with no prior programming experience who want a real path
into systems engineering, cryptography, or distributed computing.

**You will learn:**

- How computers actually work: memory, processes, files, and networks.
- The Linux command line, the shell, and everyday tooling.
- Version control with Git and collaborative workflows on GitHub.
- One first programming language (Rust is preferred, but any systems
  language is acceptable).
- How to read documentation, write small programs, and debug them.
- How to make a first open-source contribution — even a one-line fix.

**You will practice:**

- Writing small programs from scratch and reading them back.
- Using Git branches, commits, and pull requests on a real repository.
- Debugging failures by reading error messages and logs.
- Making a first pull request to an open-source project.

**After this path, you can:**

- Read and write small programs in a systems language.
- Use Git and GitHub for real collaboration.
- Understand what a process, a file descriptor, and a network socket are.
- Find and make your first useful contribution to an open-source project.

GitHub entry point: issues labeled beginner.

### 3.2 Rust / Systems Path

**For:** Developers with experience in another language who want to learn
Rust and systems programming on a real project.

**You will learn:**

- Ownership, borrowing, lifetimes, and the Rust memory model.
- Traits, generics, and type-level abstraction in idiomatic Rust.
- Error handling, testing, and the Rust toolchain (cargo, clippy,
  rustfmt) as tooling examples.
- Async programming, concurrency, executors, and runtimes — and their
  tradeoffs.
- Networking fundamentals: TCP/IP, sockets, DNS, HTTP, and the role of
  peer-to-peer transport.
- Filesystems, sockets, memory-mapped I/O, and low-level systems concepts.
- How to read and modify an existing production Rust codebase.

**You will practice:**

- Ownership and borrowing exercises.
- Writing concurrency-safe code and reasoning about its guarantees.
- Building and running tests, including property and integration tests.
- Profiling small programs to understand performance tradeoffs.

**After this path, you can:**

- Write idiomatic, tested, and safe Rust code.
- Understand memory safety, concurrency guarantees, and systems-level
  tradeoffs.
- Navigate and contribute to real Rust codebases such as Aevum or
  comparable projects.
- Reason about performance and correctness in systems software.

GitHub entry point: issues labeled rust, systems, intermediate.

---


### 3.3 Cryptography Path

**For:** People interested in cryptographic primitives, post-quantum
cryptography, and the security foundations of distributed systems.

**You will learn:**

- Cryptographic hash functions, MACs, and the properties that make them
  useful.
- Digital signatures, key exchange, and public-key infrastructure.
- Symmetric encryption, AEAD, and secure envelope design.
- Post-quantum cryptographic primitives (for example ML-KEM, ML-DSA) and
  why they matter.
- Domain separation, key hierarchies, and cryptographic context binding.
- How cryptographic primitives are used in real protocols and storage
  engines.

**You will practice:**

- Implementing toy cryptographic primitives (for learning only).
- Inspecting real protocol constructions and identifying design intent.
- Identifying common security failures in cryptographic designs.

**After this path, you can:**

- Understand cryptographic primitives, their security assumptions, and the
  limits of their use.
- Read cryptographic specifications and reason about their guarantees.
- Contribute to cryptographic components in real systems, including
  protocol and storage security layers.
- Identify common pitfalls (nonce reuse, key confusion, domain separation
  errors) in cryptographic designs.

**GitHub entry point:** issues labeled `cryptography`, `security`, `advanced`.

### 3.4 Protocol / Blockchain Path

**For:** People with prior experience in distributed systems or blockchain
who want to understand and contribute to consensus protocols, state
machines, and L1/L2 architecture.

**You will learn:**

- Distributed systems fundamentals: consistency, availability, and
  partition tolerance.
- Consensus mechanisms: PoW, PoS, BFT, and their tradeoffs.
- State machines, epochs, and deterministic execution.
- Blockchain architecture: accounts, transactions, state roots, and
  finality.
- L1/L2 separation and its implications for scalability and settlement.
- How a decentralized compute network coordinates identity, presence, and
  settlement.

**You will practice:**

- Reading and critiquing protocol specifications.
- Reasoning about liveness and safety guarantees of different consensus
  designs.
- Tracing the lifecycle of a transaction through L1 and L2 layers.

**After this path, you can:**

- Reason about the guarantees and limitations of different consensus
  designs.
- Read and critique protocol specifications.
- Understand how L1 and L2 interact in systems like Aevum.
- Contribute to protocol-level modules: state machines, epochs, settlement,
  and verification.

**GitHub entry point:** issues labeled `protocol`, `consensus`, `advanced`.

### 3.5 Compute / GPU Path

**For:** People interested in distributed compute, GPU programming, and
verification of computational workloads.

**You will learn:**

- GPU architecture: warps, memory hierarchy, and execution models.
- GPU programming models, vendor-specific platforms such as CUDA and ROCm,
  and portable compute abstractions.
- How workloads are packaged, distributed, and executed across many nodes.
- How computational results can be verified, challenged, reproduced, and
  rejected when incorrect.
- The tradeoffs between deterministic and non-deterministic workloads.
- How a compute market prices, schedules, and settles distributed work.

**You will practice:**

- Packaging a workload and running it on a compute node.
- Executing workloads across multiple nodes and comparing results.
- Designing a verification strategy for a specific workload type.

**After this path, you can:**

- Package and run workloads on GPU or CPU compute nodes.
- Understand verification strategies and their failure modes.
- Contribute to compute scheduling and verification layers in
  decentralized compute systems.
- Reason about cost, reliability, and correctness in distributed compute
  systems.

**GitHub entry point:** issues labeled `compute`, `gpu`, `verification`.

---

### 3.6 Contributor Path

**For:** Anyone who wants to move from learning to real contribution —
regardless of prior path.

This path is not a technical course. It is the path from understanding to
contribution.

**You will learn:**

- The difference between a user, a contributor, and a maintainer.
- How to find issues that match your current skill level.
- How to write a clear bug report, a feature proposal, or a documentation
  fix.
- How to work with Git branches, commits, and pull requests.
- How code review works in an open-source project — both as a reviewer and
  a reviewee.
- How to keep a contribution focused, testable, and reviewable.
- How to interact with a project's culture and conventions without
  friction.

**You will practice:**

- Making a scoped contribution to a real project.
- Reviewing someone else's pull request and providing constructive
  feedback.
- Responding to review comments on your own contribution.

**After this path, you can:**

- Navigate an open-source project's contribution flow.
- Make useful, well-scoped, and reviewable contributions.
- Review code and provide constructive feedback.
- Understand how responsibility and trust accumulate in an open project —
  up to Core Contributor and Maintainer levels.

**GitHub entry point:** issues labeled `good first issue`, `documentation`,
`help wanted`.

### 3.7 How Paths Relate

- Paths **overlap**. A person on the Compute / GPU path will likely use the
  Cryptography path's material at some point, and vice versa.
- Paths can be **entered at any point**. A developer with prior Rust
  experience can start directly at Rust / Systems or Protocol /
  Blockchain.
- Paths can be **left at any time**. There is no requirement to finish a
  path before starting another.
- The Contributor Path is **orthogonal** to the technical paths. It can be
  entered from any technical path, and it is the bridge between learning
  and doing.

A note on advancement:

> *Contributor*, *Core Contributor*, and *Maintainer* are not automatic
> promotions. They reflect accumulated trust, competence, and
> responsibility within the open project.

The paths are a map, not a contract. You choose how you move through them.

---

## 4. Learning Levels

### 4.0 Overview

Aevum Learn uses **levels** to describe the development of capability. A
level is not a rank, not a certification, and not an employment title. It is
a description of what a person can currently understand, build, and
contribute.

Levels are not a ladder you must climb. They describe where you are now, not
where you are expected to go. You can stop at any level. You can stay at any
level for years. There is nothing wrong with that.

```text
        Explorer
           ↓
        Learner
           ↓
        Builder
           ↓
       Contributor
           ↓
    Core Contributor
           ↓
      Maintainer
```

This is a model of increasing competence, contribution, trust, and
responsibility. It is not an employment or management hierarchy, and
advancement is not automatic.

4.1 What Levels Are Not

Levels are not:

· Certifications, credentials, or diplomas.
· Job titles, employment status, or HR categories.
· Public rankings or social status.
· Automatic rewards for consuming content.
· A system of gamification.

Aevum Learn does not award levels for watching videos, reading pages, or
scoring well on quizzes. Levels reflect demonstrated capability and
participation, not time spent consuming materials.

The underlying principle:

Understanding + Practice + Working results + Contribution + Earned trust
— not consumption.

4.2 The Six Levels

Explorer

What it describes: A person who is observing, reading, and exploring.

What it does not describe: A tracked or public state. Explorer is not
visible anywhere.

How it looks: Someone reading the documentation, browsing the code,
watching the project, and deciding whether decentralized compute
infrastructure is interesting.

Not tracked. Not public. No criteria.

Learner

What it describes: A person who has begun studying fundamentals —
computer science, Linux, a first systems language, Git.

How it looks: Someone writing first programs, making first commits on a
personal repository, working through first exercises.

Not tracked publicly. Personal progression only.

Builder

What it describes: A person who builds small working things — even
things no one else uses.

How it looks: Small programs that compile and run. Small scripts that
automate something. First experiments with networking, cryptography, or
async code.

Not tracked publicly. Evidence is the working artifact itself.

Contributor

What it describes: A person who has contributed to a real project —
Aevum or another open-source project.

What it does not mean: A single merged pull request is not sufficient by
itself. A typo fix and a new module are both valuable, but they do not
describe the same capability. Contributor describes a pattern of
contributions, not a count.

How it looks: A public GitHub profile with merged contributions. The
level is public as a project trust indicator — not as a badge or reward.

Visibility: Public.

Core Contributor

What it describes: A person who has demonstrated sustained technical
competence, quality of review, and reliability — and who is trusted with
scope inside the project.

How it looks: Regular, meaningful contributions; thoughtful code review;
helping other contributors; handling ambiguity.

Not automatic. GitHub metadata provides evidence, not identity.
Recognition is earned through contribution history and trust, and is
confirmed by existing Core Contributors and Maintainers.

Visibility: Public.

Maintainer

What it describes: A person who is responsible for a direction, area, or
module of the project.

What it does not mean: A manager, a boss, or an HR supervisor. A
Maintainer is responsible for something technical — a subsystem, a
specification, a piece of infrastructure — and is trusted to make decisions
about it.

How it looks: Deep familiarity with a specific area; care for its
correctness; willingness to review and mentor; long-term commitment.

Not automatic. Maintainer status is conferred through extended trust and
explicit recognition by other Maintainers.

Visibility: Public.

### 4.3 Public vs Private

| Level | Visibility | Rationale |
|---|---|---|
| Explorer | Private | Personal state, not a public claim |
| Learner | Private | Personal state, not a public claim |
| Builder | Private | Evidence is the artifact, not a label |
| Contributor | Public | Project trust indicator, useful for collaboration |
| Core Contributor | Public | Project trust indicator, useful for collaboration |
| Maintainer | Public | Project responsibility, needed by others in the project |

Public levels are **project trust levels**, not social ranking. Two people
with the same level can have very different skills, domains, and
contributions. The level describes a category of trust within the project,
not a comparison between people.

### 4.4 Progression

Progression between levels is not automatic.

- **Explorer → Learner → Builder** happen naturally as a person studies and
  builds. Nothing is tracked; nothing is claimed.
- **Builder → Contributor** happens when a person makes a real, meaningful
  contribution to a real project.
- **Contributor → Core Contributor** requires sustained contribution,
  review quality, and the trust of existing Core Contributors.
- **Core Contributor → Maintainer** requires deep responsibility and
  explicit recognition.

No level is required before entering any Learning Path (§3). Levels describe
what a person can do; Paths describe what a person wants to explore. A person
on the Cryptography Path may be at Level 1. A person at Level 4 may be on the
Compute / GPU Path. Both are normal.

Regression is also normal. A person may step away from the project for years,
change interests, or focus on a different area. Levels describe current
capability and participation, not permanent status.

---

## 5. Curriculum

### 5.0 Overview

The curriculum is the sequence of **fundamental knowledge** that underlies
decentralized compute infrastructure. It is not a catalog of courses. It is
not a list of videos. It is a map of what a person needs to understand in
order to build real things.

The curriculum is a **recommended progression**, not a mandatory gate.
Someone who already knows Rust does not need to start at Computer Science.
Someone who already knows cryptography does not need to re-study hashes.

The curriculum defines six levels of knowledge:

```text
Level 0 — Computer Science
Level 1 — Rust and Systems
Level 2 — Cryptography
Level 3 — Distributed Systems and Blockchain
Level 4 — Aevum Architecture
Level 5 — Contributor
```

5.1 The Relationship Between Levels and Paths

Level ≠ Path.

· Levels answer: "What capability level am I developing?"
· Paths answer: "What technical direction do I want to explore?"

A person may be at Level 2 with a Cryptography Path focus, or at Level 3 with
a Compute / GPU Path focus. Levels and Paths are different coordinate
systems, and they intersect freely.

A person may:

· Be at Level 3 on the Protocol / Blockchain Path.
· Be at Level 1 on the Rust / Systems Path.
· Contribute on both in the same month.

This is normal. Levels and Paths are not the same thing, and they are not
required to be aligned.

5.2 Level 0 — Computer Science

Focus: How computers actually work.

Topics:

· Memory, processes, threads, and files.
· Linux fundamentals: shell, permissions, processes, filesystems.
· Networking basics: IP, TCP, DNS, HTTP.
· Git and version control as a daily tool.
· A first programming language (Rust preferred; another systems language is
  acceptable).
· Reading and writing documentation.
· Debugging basics: reading errors, reading logs, isolating failures.

Practice: Writing small programs, running them, and breaking them on
purpose to understand what happens.

Suggested GitHub entry point: beginner, documentation.

5.3 Level 1 — Rust and Systems

Focus: Writing real systems code.

Topics:

· Ownership, borrowing, and lifetimes.
· Traits, generics, and idiomatic Rust abstraction.
· Error handling and testing.
· Async programming, concurrency, executors, runtimes, and their tradeoffs.
· Filesystems, sockets, and low-level I/O.
· Networking fundamentals in code: TCP, HTTP, peer-to-peer transport.
· Reading and modifying an existing Rust codebase.

Practice: Writing programs that use concurrency safely, run tests,
profile performance, and integrate with an existing codebase.

Suggested GitHub entry point: rust, systems, intermediate.

---


### 5.4 Level 2 — Cryptography

**Focus:** The cryptographic foundations of secure systems.

**Topics:**

- Hash functions and MACs.
- Digital signatures and key exchange.
- Symmetric encryption and AEAD.
- Post-quantum primitives (for example ML-KEM, ML-DSA).
- Domain separation, key hierarchies, and cryptographic context binding.
- Common pitfalls: nonce reuse, key confusion, domain separation errors.
- How cryptographic primitives are used in protocols and storage engines.

**Practice:** Implementing toy primitives for learning, inspecting real
constructions, and identifying design failures.

**Suggested GitHub entry point:** `cryptography`, `security`, `advanced`.

### 5.5 Level 3 — Distributed Systems and Blockchain

**Focus:** Coordination without a central authority.

**Topics:**

- Distributed systems fundamentals: consistency, availability, partition
  tolerance.
- Consensus mechanisms: PoW, PoS, BFT, and tradeoffs.
- State machines and deterministic execution.
- Blockchain architecture: accounts, transactions, state roots, finality.
- L1/L2 separation and settlement.
- Coordination of identity, presence, settlement, and programmable
  economics.

**Practice:** Reading protocol specifications, reasoning about safety and
liveness, tracing transactions through layers.

**Suggested GitHub entry point:** `protocol`, `consensus`, `advanced`.

### 5.6 Level 4 — Aevum Architecture

**Focus:** The specific architecture of Aevum and its components.

**Topics:**

- Aevum L1 protocol: identity, presence, settlement.
- Aevum L2 execution environment and compute market.
- Compute Engine: workload packaging, scheduling, verification.
- Storage: AevumDB, envelope crypto, domain separation.
- Post-quantum cryptography in Aevum.
- Verification strategies: redundancy, deterministic recomputation,
  probabilistic checks.
- Cross-module interactions: how L1, L2, compute, and storage fit together.

**Practice:** Reading Aevum source code, tracing flows across modules, and
proposing improvements grounded in the actual architecture.

**Suggested GitHub entry point:** `protocol`, `compute`, `verification`,
`advanced`.

### 5.7 Level 5 — Contributor

**Focus:** Turning capability into real contribution.

This level is not a technical course. It is the operational level where a
person moves from studying to participating in a real project.

**Topics:**

- Finding issues that match your current skill level.
- Writing clear bug reports, feature proposals, and documentation fixes.
- Git branches, commits, and pull requests.
- Code review as reviewer and reviewee.
- Keeping contributions focused, testable, and reviewable.
- Interacting with project culture and conventions.

**Practice:** Making scoped contributions to real projects; reviewing real
pull requests; responding to review comments.

**Suggested GitHub entry point:** `good first issue`, `documentation`,
`help wanted`.

### 5.8 Anti-Gamification

The curriculum does not award levels for consuming content. The following do
not determine a level:

- Videos watched.
- Pages read.
- Quiz scores.
- GitHub activity counts.
- Number of pull requests alone.
- Time spent on the platform.

Levels reflect demonstrated capability and participation. A person's level
is a description of what they can do, not a score of what they have seen.

### 5.9 Curriculum vs Paths

The curriculum is a **sequence of capability**. The paths (§3) are a **map of
directions**. They intersect freely.

- A person can progress through Levels 0–5 on any Path.
- A person can explore any Path at any Level.
- A person may focus on one Path and advance through several Levels within
  it.
- A person may switch Paths at any time.

The curriculum defines *what to learn*. The paths define *where to apply it*.

---

## 6. Contributor Pathway

### 6.0 Overview

Section 4 defined the six levels. This section explains how contribution
actually becomes trust inside Aevum.

The Contributor Pathway is not a career ladder. It is not a hiring funnel. It
is not a promotion process. It is a description of how an open project
recognizes the people who build it.

The guiding question:

> **How does contribution become trust?**

The answer is not voting, not metrics, and not time served. The answer is
demonstrated work and demonstrated responsibility, recognized by others who
already carry that responsibility.

Trust is contextual. A person may be trusted in one area and still be
learning in another.

### 6.1 What Counts as Contribution

Contribution is not limited to code. Code is one kind of contribution; it is
not the only kind, and it is not privileged above the others.

Contribution includes:

- **Code** — features, fixes, refactors, tests, tooling.
- **Documentation** — guides, specifications, architecture notes, translations.
- **Review** — reading pull requests carefully, giving useful feedback,
  catching issues.
- **Testing** — manual testing, integration testing, reproducing bugs,
  writing test coverage.
- **Security research** — responsible disclosure, threat modeling,
  cryptographic review.
- **Protocol research** — consensus, settlement, verification, economic
  design.
- **Education** — writing learning materials, mentoring, explaining concepts
  clearly.
- **Translation** — making knowledge accessible in other languages.
- **Moderation** — keeping community channels useful and welcoming.
- **Infrastructure operations** — running nodes, providing test
  environments, operating public infrastructure.
- **Workload development** — authoring, packaging, and verifying
  computational workloads.
- **Community support** — helping newcomers, answering questions, triaging
  issues.

A person who only writes code is not more of a contributor than a person who
only writes documentation. Both are essential to the project.

### 6.2 From Builder to Contributor

A Builder becomes a Contributor when they make a real, meaningful
contribution to a real project — Aevum or another open-source project.

There is no application. There is no gate. There is no minimum count.

A single merged pull request may be a first contribution. It is not, by
itself, a description of capability. Contributor is not a title awarded for a
first merge. It is a description of a pattern.

For Learn purposes, evidence is public and may include merged contributions,
reviews, documentation, research, infrastructure work, or other visible
contributions — not only code.

When it becomes visible, the level functions as a **project trust
indicator** — useful for collaboration, not for ranking.

### 6.3 From Contributor to Core Contributor

A Contributor becomes a Core Contributor when they have demonstrated
**sustained technical competence, quality of review, and reliability**, and
are trusted with scope inside the project.

The pathway:

- Recognition is earned through sustained contribution and trust.
- It is not requested, applied for, or automatically granted.
- It is not decided by vote counts, percentages, or activity metrics.
- It is offered by existing Core Contributors and Maintainers, on the basis
  of:
  - contribution history;
  - quality of work;
  - quality of review;
  - how the person interacts with others;
  - willingness and ability to take responsibility;
  - judgement in ambiguous situations.

> **Recognition is earned through sustained contribution and trust. It is
> not requested, applied for, or automatically granted.**

When recognition happens, it is made public — so that other contributors know
who they can rely on for review, guidance, and responsibility.

### 6.4 From Core Contributor to Maintainer

A Core Contributor becomes a Maintainer when they take **long-term
responsibility for a specific area, subsystem, or specification** of the
project.

A Maintainer is not a manager. A Maintainer is responsible for something
technical — a module, a spec, a piece of infrastructure — and is trusted to
make decisions about it.

Maintainer status is conferred through extended trust and explicit
recognition by other Maintainers.

Once again:

- Not automatic.
- Not requested.
- Not based on activity metrics.
- Not a management position.

### 6.5 No Revocation

Contributor, Core Contributor, and Maintainer status is **historical**. It
reflects what a person has contributed and what trust they have earned.

If a person steps away from the project:

- Their **historical contribution remains part of the project's history**.
- Their **level is not revoked** because of inactivity.
- Their **responsibility for a specific area may be reassigned** if they are
  no longer active.

Example: a Maintainer who is no longer maintaining their area may hand
responsibility to another Maintainer. Their recognition as a Maintainer
remains; their active responsibility changes.

This is different from a leaderboard. There is no seasonal reset. There is no
score that decays. There is no gamification.

### 6.6 Public Recognition

Contributor, Core Contributor, and Maintainer are **public levels** (§4.3).

Public recognition serves a practical purpose:

- Other contributors know who to ask for review or guidance.
- New contributors can see the project's structure of trust.
- Responsibility is visible, so it can be relied on.

Public recognition is not a badge, a status symbol, or a ranking. Two
Contributors may have very different skills, domains, and levels of
activity. The level describes a category of trust, not a comparison between
people.

---

## 7. Mentorship

### 7.0 Overview

Section 5 defined the curriculum. This section explains how knowledge
actually becomes capability inside the project.

The guiding question:

> **How does knowledge become capability?**

The answer is not courses, not certifications, and not scheduled lectures.
The answer is mentorship — informal, voluntary, and continuous.

Mentorship is not a program that Aevum runs. It is a culture that Aevum
maintains.

### 7.1 Free-Form and Voluntary

Mentorship in Aevum is:

- **Free-form.** No assigned pairs. No mandatory sessions. No KPI.
- **Opt-in.** A person asks when they need help. A person offers help when
  they can.
- **Voluntary.** Nobody is required to mentor. Nobody is required to be
  mentored.
- **Reciprocal.** Today's mentee is often tomorrow's mentor.

Mentorship is not something scheduled into a calendar. It arises naturally
around shared work.

### 7.2 Where Mentorship Happens

Mentorship happens wherever learning happens. This includes:

- **Discord** — for fast questions and quick feedback.
- **GitHub Discussions** — for long-lived knowledge that benefits future
  readers.
- **Pull request reviews** — for learning through real code and real review.
- **Documentation** — for learning through writing and reading structured
  material.
- **Community channels** — for general questions, translations, and support.

> **Mentorship happens wherever learning happens.**

Well-written documentation is a form of mentorship. It allows one contributor
to help thousands of future readers.

There is no single "mentorship channel". The whole project is the mentorship
channel.

### 7.3 Teach Reasoning, Not Answers

The most important principle of mentorship in Aevum:

> **Teach reasoning, not answers.**

The goal of mentorship is not to create dependency. The goal is to help a
person become capable of solving future problems independently.

A mentor does not hand over a finished solution. A mentor helps a person:

- understand the problem;
- reason about the constraints;
- find the information they need;
- read the relevant specification;
- check a hypothesis;
- design and test a solution.

The value of mentorship is not the answer. The value is the ability to find
the answer — and to know when the answer is wrong.

### 7.4 RTFM Is Not a Mentorship Answer

In many technical communities, the response to a beginner's question is
**"RTFM"** — read the manual.

In Aevum, this is not a mentorship answer.

A person who asks a question may have already read the manual — or may not
know where the manual is, or may be stuck on a specific point that the manual
assumes is obvious. Telling them to read the manual again does not teach
anything.

The correct response is to:

- answer the question, or
- point to the exact section of documentation that answers it, or
- ask what they have tried, so the conversation can continue.

Senior contributors are expected to explain, not dismiss.

### 7.5 Language

English is the primary language of documentation and technical discussion.

At the same time:

> **Contributors are free to help each other in any language.**

A global project benefits from multilingual support. Documentation is written
in English so that it can serve as a shared reference. Person-to-person help
can happen in whatever language both sides prefer.

### 7.6 No Scheduled Program

Aevum does not run a scheduled mentorship program. There are no assigned
mentors, no cohorts, no reviews, no graduation ceremonies.

Instead, mentorship emerges from:

- Asking good questions in public channels.
- Answering questions carefully and respectfully.
- Doing real work together in pull requests and discussions.
- Writing documentation that helps future readers.
- Reviewing others' work honestly and constructively.

The role of the project is to maintain the culture that makes this possible —
not to schedule it.

### 7.7 What Mentorship Is Not

Mentorship is not:

- A scheduled course.
- A guarantee of individual attention.
- A promise of promotion.
- A financial arrangement.
- An employment relationship.

Mentorship is a voluntary culture of learning. It exists as long as
contributors choose to keep it alive.

---

## Document Status

Sections 1 through 7 are frozen. Remaining sections (§8 GitHub Integration
through §15 Future Academy Evolution) will be added in subsequent
iterations.
