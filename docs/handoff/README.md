# Handoffs

Every build step ends with a handoff note in this folder, written for the next agent or person who
picks the work up, including a partner working in parallel on another branch.

- **One file per step**, named `YYYY-MM-DD-<topic>.md`. Do not edit an old one except to fix a fact.
- Open with *where things stand*, then *what this step did*, *findings*, *decisions*, *open
  questions*, *the state of every branch*, *next steps in order*, and *how to check*.
- Say what was **verified** (with the CI run or test that shows it) and what was only **read or guessed**.
- Before starting, read the newest handoff on `main` **and** on any open branch
  (`git ls-tree -r origin/<branch> docs/handoff`).

| Step | Note | Agent |
|---|---|---|
| Face-pull test | [2026-11-11-face-pull.md](2026-11-11-face-pull.md) | Claude (session 1) |
