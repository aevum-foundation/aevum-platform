# Aevum Legal Pages — Design Contract v1

**Status:** Draft
**Last updated:** 2026-09-17

---

## 1. Purpose

Define the design and content rules for legal pages
(Privacy Policy, Terms of Service).

---

## 2. Scope

Covers:

- privacy.html
- terms.html

Does not cover:

- Documentation (Docs)
- Learning (Learn)
- Operational pages

---

## 3. Pages

| Page | Type | Purpose |
| ---- | ---- | ------- |
| privacy.html | Legal | Privacy Policy |
| terms.html | Legal | Terms of Service |

---

## 4. Content Principles

### 4.1 Accuracy of Legal Statements

Legal pages must describe verified production behavior.
If a behavior is uncertain or environment-dependent,
the wording must avoid presenting it as a universal fact.

### 4.2 Third-Party Services

Legal pages must distinguish Aevum-controlled services
from third-party platforms such as GitHub and Telegram.

Third-party services are governed by their own terms
and privacy policies.

### 4.3 Plain Language

- Describe the system that actually exists, not the system
  we intend to build.
- Do not make absolute promises that cannot be verified.
- Separate the protocol from the web platform.
- Do not reveal internal implementation details
  (e.g., Argon2id) in legal text.
- Use plain language where possible.

---

## 5. Layout Rules

- Docs-like, but without Docs sidebar.
- Shared header and footer.
- Readable column: max-width 760-820px.
- No CTA, no cards, no decorative elements.
- Standard H2/H3 hierarchy.

---

## 6. Typography

- Same as site (theme.css, app-shell.css).
- Body: 16-18px.
- Line-height: 1.6-1.7.

---

## 7. Color

- Same tokens as site (theme.css).
- No accent colors for legal content.

---

## 8. Components

Allowed:

- Shared header
- Shared footer
- Standard headings (h1, h2, h3)
- Paragraphs
- Lists
- Tables (if needed for data categories)

Forbidden:

- CTA buttons
- Cards
- Decorative elements
- Marketing components

---

## 9. Accessibility

- Correct document language metadata (`lang`)
- Visible heading hierarchy
- Sufficient text contrast
- Keyboard-accessible navigation
- Semantic HTML
- ARIA where needed

---

## 10. Privacy Policy Structure

13 sections:

1. Privacy Policy
2. Scope and Applicability
3. Information We Collect
   3.1 Account Information
   3.2 Authentication and Security Information
   3.3 Profile and Preferences
   3.4 Technical Information
4. Information We Do Not Require
5. How We Use Information
6. Cookies and Local Storage
7. Data Storage and Security
8. Third-Party Services
9. Data Retention and Deletion
10. Your Rights
11. International Data Transfers
12. Changes to This Policy
13. Contact

---

## 11. Terms Structure

TBD after Privacy is finalized.

---

## 12. Checklist for Legal Pages

- [ ] Describes actual system
- [ ] No unverifiable promises
- [ ] Protocol / platform separated
- [ ] No internal implementation details
- [ ] Shared header/footer
- [ ] Readable column
- [ ] No CTA/cards
- [ ] Accessibility verified

---

## 13. When to Update This Contract

- When a new legal page is added.
- When data collection changes.
- When analytics or third-party services change.
- When layout rules change.
