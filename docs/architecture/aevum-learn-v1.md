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

## Document Status

Sections 1, 2, and 3 are frozen. Remaining sections (§4 Learning Levels
through §15 Future Academy Evolution) will be added in subsequent
iterations.
