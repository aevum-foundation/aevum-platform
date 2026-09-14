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

## Document Status

Sections 1 and 2 are frozen. Remaining sections (§3 Learning Paths through
§15 Future Academy Evolution) will be added in subsequent iterations.
