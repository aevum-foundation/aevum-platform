# Contributing to Aevum Platform

Welcome. This document describes how to contribute to the Aevum Platform — the web platform, API, authentication subsystem, and community-facing code.

Aevum is an open project. You do not need permission to contribute. You do not need to be a member of any organization. You do not need to make a long-term commitment. You can make one pull request and stop, or contribute for years. Both are normal.

If you have not read the Learn documentation yet, start at:

- **Learn v1** — `https://aevumchain.com/learn`
- **Architecture docs** — `docs/architecture/`

---

## Scope of Public Contributions

Aevum maintains several repositories.

**Public repositories:**

- **`aevum-platform`** — the web platform, REST API, authentication subsystem, and community-facing code. **This is the repository you are looking at now.** Contributions to the website, API, auth, and community features happen here.
- **`aevum-public`** — the public portion of the protocol: specifications, architecture documents, and public reference material for L1, consensus, Presence, JT-UTXO, and post-quantum components. Contributions to public protocol documentation and specifications happen there.

**Non-public repositories:**

- **`aevum-protocol`** — the core L1 protocol implementation, consensus, and cryptography.
- **`aevum-db`** — the universal storage engine and post-quantum cryptography layer.

**Public repositories are the only repositories that accept external contributions.**

Not all development happens in public repositories. This is normal engineering practice. The core protocol and the storage engine contain technology that is intentionally kept private at this stage.

The existence of private repositories does not reduce the value of public contributions. Public contributions are the primary path through which external contributors participate.

If you are unsure whether something belongs in the public scope, ask in [GitHub Discussions](https://github.com/aevum-foundation/aevum-platform/discussions) or see the Community page on `aevumchain.com`.

**Protocol-related contributions** (L1, consensus, Presence, JT-UTXO, post-quantum) belong in `aevum-public`. See the contribution guide in that repository for details.

---

## Before You Start

To contribute to `aevum-platform`, you will typically need:

- **Git** — for version control.
- **Rust** — for backend and API work. See [The Rust Book](https://doc.rust-lang.org/book/) if you are new to Rust.
- **A GitHub account** — for opening issues and pull requests.

You do not need:

- prior experience with decentralized systems;
- prior experience with cryptography;
- a formal degree;
- permission from anyone.

If you are new to programming or to Rust, [Learn v1](https://aevumchain.com/learn) describes a path from fundamentals to contribution.

---

## How to Find Work

Project work is primarily tracked in GitHub issues.

Issues are labeled with:

- **Level labels** — `level:beginner`, `level:intermediate`, `level:advanced`, `level:research`
- **Domain labels** — `rust`, `api`, `auth`, `community`, `documentation`, `infra`, `testing`, `security`
- **Status labels** — `good first issue`, `help wanted`, `discussion`, `blocked`

Level labels are **navigation hints**, not gates. You can pick any issue you feel ready for.

### Good first issue

A `good first issue` is a task that is:

- well-scoped;
- isolated from complex subsystems;
- understandable without deep prior knowledge;
- low-risk;
- verifiable.

Good first issues are the recommended starting point for new contributors. They are not the only starting point.

---

## How to Make a Contribution

### 1. Pick an issue

Choose an issue that interests you. If an issue is already assigned to someone, ask before starting work on it.

### 2. Fork and clone

Fork the repository on GitHub, then clone your fork:

```bash
git clone https://github.com/<your-username>/aevum-platform.git
cd aevum-platform/backend
```

### 3. Create a branch

Use a short, descriptive branch name:

```text
feat/add-backup-codes
fix/session-timeout
docs/learn-links
```

This is a recommendation, not a rule.

### 4. Make the change

Keep the change focused. Do not include unrelated work in the same pull request.

### 5. Run the checks relevant to your change

Verification is proportional to change.

For Rust code, typically:

```bash
cargo fmt --check
cargo test
cargo clippy
```

For documentation or HTML/CSS changes, no Rust checks are required.

If you are unsure what to run, ask.

### 6. Commit

Use a short, descriptive commit message. Common prefixes:

```text
feat: ...
fix: ...
docs: ...
refactor: ...
test: ...
chore: ...
```

This is a convention, not a strict requirement.

---

## Opening a Pull Request

A pull request description should answer three questions:

- **What changed?** — one or two sentences.
- **Why?** — link to the related issue or explain the motivation.
- **How was it verified?** — which checks you ran.

Keep pull requests focused. **One issue → one pull request.**

Small, focused pull requests are easier to review and easier to merge. Large pull requests that touch unrelated parts of the codebase are usually asked to be split.

For non-trivial changes, open or reference an issue first when practical. Small fixes — typos, small documentation corrections, obvious bug fixes — do not require a prior issue.

---

## Code Review

Pull requests are reviewed before they are merged. Review is a conversation, not a gate.

What to expect:

- A reviewer may ask questions, suggest changes, or point out issues.
- This is normal. It is not a judgment of you or your work.
- You can ask for clarification on any review comment.
- You can disagree, as long as you explain your reasoning.

How to review others:

- Read the change carefully.
- If something is unclear, ask.
- Suggest improvements, not just problems.
- Be specific. "This looks wrong" is less useful than "This will panic if items is empty".
- Remember that the person on the other side is also learning.

**"RTFM" is not a review comment.** If someone asks a question, answer it — or point to the exact section of documentation that answers it. See Learn v1 §7.4 for the mentorship culture.

---

## Non-Code Contributions

Code is one kind of contribution. It is not the only kind, and it is not privileged above the others.

Other ways to contribute to `aevum-platform`:

- Documentation — guides, API docs, corrections, examples.
- Translation — making content accessible in other languages.
- Review — reading pull requests and giving useful feedback.
- Testing — reproducing bugs, writing test coverage, manual testing.
- Security research — responsible disclosure of vulnerabilities.
- Education — writing learning material, mentoring, explaining concepts.
- Moderation — keeping community spaces usable.
- Infrastructure — running test environments, improving CI.
- Community support — helping newcomers, answering questions.

All of these are contributions.

---


## Security Disclosure

**Do not open public issues for security vulnerabilities.**

If you believe you have found a security issue in `aevum-platform`, report it privately.

See `SECURITY.md` in this repository for the current disclosure process.

Public issues about security are closed and redirected to the private channel.

---

## AI-Assisted Contributions

AI-assisted contributions are allowed.

If you use AI tools to help write code, documentation, or tests:

- **You remain responsible for the result.** The contributor is accountable for correctness, testing, licensing, and security.
- **You should understand the code you submit.** If you cannot explain what your change does, do not submit it.
- **You should verify the output.** AI tools generate plausible-looking code that is sometimes wrong. Run the tests. Read the diff.
- **You should respect licenses and attribution.** Do not submit code you do not have the right to submit.

AI tools are tools. The contribution is yours.

---

## Getting Help

If you are stuck, ask.

- **GitHub Discussions** — for longer technical questions and design discussions: `https://github.com/aevum-foundation/aevum-platform/discussions`
- **GitHub Issues** — for bug reports and feature requests.
- **Community page** — see `aevumchain.com` for the current list of community channels.

English is the primary language of documentation and technical discussion. Contributors are free to help each other in any language.

---

## Recognition

Contributors who make sustained, high-quality contributions are recognized as Contributors, Core Contributors, or Maintainers.

This is not a promotion process. It is not automatic. It is not based on metrics. Recognition is offered by existing Core Contributors and Maintainers, based on contribution history, quality of work, quality of review, and trust.

See **Learn v1 §6 (Contributor Pathway)** for details.

---

## What Not to Do

- **Do not include unrelated changes in the same pull request.** One change, one PR.
- **Do not argue with review comments.** Ask questions, explain your reasoning, but do not treat review as a conflict.
- **Do not open public security issues.** See the Security Disclosure section above.
- **Do not spam issues, pull requests, or discussions.** Quantity is not quality.

---

## Related Documents

- **Learn v1** — `https://aevumchain.com/learn`
- **Architecture docs** — `docs/architecture/`
- **Code of Conduct** — `CODE_OF_CONDUCT.md`
- **Security policy** — `SECURITY.md`
- **Protocol contribution guide** — `aevum-public/CONTRIBUTING.md` *(for L1, consensus, JT-UTXO, post-quantum)*
- **GitHub Discussions** — `https://github.com/aevum-foundation/aevum-platform/discussions`

---

Aevum is built by people who choose to build it. Thank you for considering a contribution.
