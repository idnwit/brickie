# Exact module members

## Decision

Add `module:<id>` as an exact module selector. Keep bare module IDs as prefix
selectors, package selectors as all targets of a package, and `path:` selectors
as package-directory selectors. Specificity is path < package < module prefix <
exact module. No cache schema change is needed: mapping is resolved at check/view
time from existing module facts.

## Motivation

Nested consumer APIs and implementations can share a source parent while
independent descendants belong elsewhere. Prefix-only membership cannot isolate
the parent without reaching outside a composite. Exact membership describes this
partition without moving source or disabling outside-parent validation. It also
isolates the SDK root module from all other targets of the same package.

## Invariants

- Exact selectors match only the specified module, never descendants or alternate
  package targets.
- Legacy selectors and specificity remain unchanged.
- Unassigned, overlapping and outside-parent selections remain errors; exact
  membership is not an exclusion or a way around partition validation.
- Unknown/inactive module IDs remain unmatched, consistent with existing feature
  and target handling.

## Verification and review

Focused selector tests cover roots, descendants, alternate targets and precedence.
A nested resolution regression covers an exact parent with independently assigned
descendants. Existing partition/error tests remain in the suite.

Verified in the isolated extension checkout: formatting, workspace clippy with
warnings denied, workspace tests, and release CLI build. Two independent reviews
checked selector/partition correctness and a consuming project's nested policy.
The selector review identified a test construction error, which was corrected;
final review found no blocking issues. The consuming graph resolved its intended
composites while preserving real dependency and complexity findings.
