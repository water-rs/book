# AGENTS.md

Guidance for coding agents (Claude Code, Codex, and friends) working in this
repository. `CLAUDE.md` is a symlink to this file — keep both in sync by
editing only this one.

This repository hosts the **WaterUI Tutorial Book** — an mdBook that teaches
WaterUI from first principles through advanced internals. The book is read by
real users and rendered to <https://book.waterui.dev>; correctness and clarity
matter more than volume.

---

## 1. Source of Truth

The `waterui/` directory is a **git submodule** pinned to a specific commit on
the upstream `dev` branch (`https://github.com/water-rs/waterui.git`).

- **Every API name, type, signature, module path, feature flag, CLI subcommand,
  or example shown in the book MUST be verifiable against the currently pinned
  waterui commit.** Read the actual Rust source under `waterui/` instead of
  relying on memory or external docs.
- `nami` is **not** a top-level submodule of this book. It lives inside waterui
  at `waterui/vendor/nami` and is re-exported through `waterui::reactive` /
  `waterui::Signal` / `waterui::Binding`. Never re-add a top-level `nami`
  submodule and never reference a `nami_*` crate path that bypasses the
  re-export.
- The `waterui` crate at the workspace root re-exports everything users need
  through `waterui::prelude::*`. Examples should prefer the re-export path
  (`waterui::layout::stack::vstack`) over the inner crate path
  (`waterui_layout::stack::vstack`) unless the chapter is specifically teaching
  the workspace topology.

### Submodule pin = book version

The committed submodule SHA is the book's *de facto* version stamp. Any chapter
checked into the same commit as the submodule pin must describe that pinned
waterui truthfully. When you bump the submodule, you take responsibility for
re-auditing every chapter that touches APIs that changed between the old and
new pin.

### How to bump the submodule

```bash
# 1. Fetch the latest dev tip and update the working tree
git submodule update --remote --init --recursive waterui

# 2. Audit chapters for drift (see Section 5)

# 3. Stage and commit the new SHA together with any chapter fixes
git add waterui src/
git commit -m "Bump waterui submodule to <short-sha> + chapter sync"
```

Never commit a submodule bump without running the audit, and never commit
chapter changes that depend on an unbumped submodule.

---

## 2. Repository Layout

```
book/
├── AGENTS.md             # this file (CLAUDE.md is a symlink to it)
├── README.md             # public-facing repo readme
├── book.toml             # mdBook config
├── Cargo.toml            # workspace root for runnable example crate
├── src/                  # book source (markdown + tests)
│   ├── SUMMARY.md        # table of contents — single source of truth for ordering
│   ├── Introduction.md
│   ├── 01-getting-started/
│   ├── 02-core/
│   ├── 03-ui/
│   ├── 04-rich/
│   ├── 05-graphics/
│   ├── 06-advanced/
│   ├── 07-tools/
│   ├── 08-internals/
│   ├── 09-philosophy.md
│   ├── appendix/
│   └── lib.rs            # canonical, compile-tested example snippets
├── scripts/              # mdbook helpers (custom rustdoc wrapper)
└── waterui/              # submodule — DO NOT edit
```

**Do not add or rename top-level directories without updating this section.**
Future agents read this map to navigate.

### Off-limits

- `waterui/` and any path inside it (it is upstream code; modifications are
  silently lost on the next submodule update).
- `book/` (build output, gitignored).
- `target/` (Cargo output, gitignored).
- `.github/workflows/deploy.yml` — the deploy pipeline is intentionally
  minimal. Do not add CI logic, preflight scripts, or workspace-wide formatters
  there. If a workflow change is genuinely required, surface it for review
  rather than slipping it into an unrelated PR.

---

## 3. Chapter Authoring Rules

### Voice and structure

- **Address the reader directly.** Use "you", not "we" or "the user".
- **Open every chapter with a `> **In this chapter, you will:**` callout** of
  3–5 bullet points stating concrete learning outcomes.
- **Lead with the why before the how.** A few sentences of motivation, then
  the smallest possible runnable example, then progressive depth.
- **Prefer working examples over prose paragraphs.** Code blocks are the
  primary teaching surface; prose explains intent and edge cases.
- Use sentence-case headings, not Title Case.
- One H1 per file (the chapter title). Use H2 for sections, H3 for subsections.
- Cross-link to other chapters with relative markdown links
  (`[the View chapter](../02-core/01-view.md)`).

### Code samples

- Every Rust code block intended for compilation must use a triple-backtick
  ` ```rust ` fence. Use ` ```rust,ignore ` only for snippets that intentionally
  cannot stand alone (e.g., shader source, partial API sketches).
- Imports must be explicit. Prefer `use waterui::prelude::*;` at the top of a
  block, then add specific imports for items not covered by the prelude.
- Cite the real type, real method, real signature from the pinned submodule.
  When in doubt, open the file under `waterui/` and read it.
- If an API is feature-gated, name the feature in prose
  (e.g., "requires the `chart` feature").
- Never invent `unwrap()` chains, panicking helpers, or "TODO: real impl"
  placeholders to make a sample look complete.

### Length and depth

- Aim for one concept per chapter. If a chapter exceeds ~600 lines or covers
  three unrelated subsystems, split it.
- Each chapter should be self-sufficient enough that a reader can land on it
  from a search engine and still get value, but not so repetitive that
  sequential reading feels redundant. A short "see also" footer is preferred
  over re-explaining shared concepts.

### What not to write

- No fabricated benchmarks, no fabricated quotes, no fabricated platform
  support claims.
- No marketing fluff ("blazing fast", "revolutionary", "industry-leading").
  Describe what the framework does and let the reader judge.
- No emoji in chapter bodies unless illustrating a concrete UI affordance.
- No "Conclusion" or "Summary" sections that just restate the chapter — end
  on the next concrete action (link to the following chapter, suggested
  exercise, or pointer to deeper reading).

---

## 4. Build and Test

```bash
# Render the book locally
mdbook build

# Live-reload preview
mdbook serve --open

# Compile the canonical example crate (verifies src/lib.rs against the pinned waterui)
cargo build

# Run example unit tests
cargo test
```

The deploy workflow only runs `mdbook build`; it does not compile Rust code
blocks. **`cargo build` is therefore the most reliable smoke test that the
book's API claims still match the pinned waterui** — run it before claiming a
chapter rewrite is done.

If you change `src/lib.rs`, you are also changing the contract for every
chapter that links to those snippets — update both together.

---

## 5. Audit Workflow (when bumping the submodule)

Run this whenever you change the pinned waterui SHA, when a chapter feels
stale, or when a reader reports a broken sample.

1. **Diff the upstream changes.**
   ```bash
   cd waterui && git log --oneline OLD_SHA..NEW_SHA -- core/ components/ macros/ cli/
   ```
   Pay special attention to changes under `core/`, `components/*/`, `macros/`,
   `facade/`, and `cli/` — these surface in user-facing APIs.

2. **For each touched module, grep the book for references.**
   ```bash
   rg -n 'Binding::|Computed::|vstack|hstack|button\(|text!|navigation|TabView' src/
   ```
   Resolve each hit against the new code.

3. **Run `cargo build` and `cargo test`.** Compilation failures pinpoint
   chapters that still reference removed or renamed APIs.

4. **Render the book** with `mdbook build` and click through any
   automatically generated cross-links to confirm nothing is dangling.

5. **Update `src/lib.rs`** to keep the canonical snippets in sync. Chapters
   should preferentially link/quote from `src/lib.rs` so the compiler enforces
   correctness.

6. Commit the submodule bump and the chapter changes **in the same commit**.
   Reviewers must be able to see, in one diff, which API rename caused which
   prose change.

---

## 6. Engagement Rules for Agents

- **Do not edit `waterui/`.** It is a submodule. Edits are silently discarded
  by the next `submodule update`. If you need an upstream change, stop and
  surface the request to the user.
- **Do not run workspace-wide formatters or refactors** (`cargo fmt --all`,
  bulk codemods, sweeping search-replace). Touch only the files the task
  requires.
- **Do not add fallback paths, shims, or "simpler approach" workarounds.**
  Either the chapter accurately describes the pinned waterui or it must be
  fixed. Never paper over an API mismatch by softening the prose.
- **Do not invent APIs to make a chapter feel complete.** If the framework
  doesn't yet expose what the chapter needs, document the actual current
  surface and surface the gap rather than fabricating a method name.
- **Do not commit binary assets** (large images, screenshots, recordings)
  without first checking that they are essential. Prefer ASCII diagrams and
  external links.
- **Respect the submodule pin as immutable history.** Never `git checkout` the
  submodule to a different SHA without the user's go-ahead, and never use
  `--force` flags to bypass git safety checks.
- **No `println!` / `print!` / `eprintln!` in `src/lib.rs` or chapter Rust
  examples.** Use `tracing::info!` / `tracing::debug!` so samples model the
  real-world WaterUI logging convention.
- **Multi-line embedded text belongs in a separate file**, included via
  `include_str!`. Do not paste shaders, large markdown, or template strings
  inline in `.rs` examples.
- **Begin every response with a respectful greeting to the user**, in the tone
  of a loyal servant addressing their master. This canary signals that
  context is intact.

---

## 7. When You Are Stuck

- If `cargo build` fails with an error you cannot resolve from the pinned
  submodule alone, **stop and ask the user**. Do not start patching upstream
  code, do not weaken the example to silence the error, do not paste
  speculative API names.
- If a chapter is clearly out of date but the correct shape of the new API is
  ambiguous, **propose two options to the user** with concrete code sketches
  rather than picking arbitrarily.
- If you encounter unfamiliar files, branches, or working-tree state,
  investigate before deleting or overwriting — it may be the user's
  in-progress work.

---

## 8. Pointers

- Upstream WaterUI repository: <https://github.com/water-rs/waterui>
- Upstream `AGENTS.md` (read before deep architectural chapters):
  `waterui/AGENTS.md`
- Public book site: <https://book.waterui.dev>
- mdBook user guide: <https://rust-lang.github.io/mdBook/>
