# Contributing to ModernLink

Thank you for contributing. ModernLink bridges legacy Java applications to modern
protocols, so small changes can affect compatibility, delivery guarantees, and
security. Please keep each contribution focused and evidence-backed.

## Before opening a pull request

1. Open an issue first for a substantial feature, behavior change, or design decision.
   Describe the problem, intended outcome, and affected compatibility contract.
2. Work on a dedicated branch and keep unrelated changes out of the pull request.
3. Add or update focused tests with behavioral changes. A failing regression test should
   demonstrate the issue before the fix.
4. Run the relevant checks and report their actual results and limits in the pull-request
   description. A successful command is evidence, not proof of an untested runtime contract.
5. Review the rendered diff yourself. Do not include generated or unrelated files.

## Required project conventions

- Use Conventional Commit prefixes: `feat:`, `fix:`, `docs:`, `build:`, `style:`, or `merge:`.
- Java facade code under `java/src` must remain Java 6 compatible.
- Unsupported delivery guarantees must fail explicitly; never silently degrade behavior.
- Do not commit secrets, broker endpoints, credentials, or message payloads.
- Keep Rust provider implementations behind the provider-neutral messaging boundary.

The full setup, validation commands, Java-specific pre-check, code conventions, and
verification caveats are maintained in [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md).
Read [docs/VERIFICATION.md](docs/VERIFICATION.md) before claiming that an integration,
broker, Java, or native behavior has been proven.

## AI-assisted contributions

Tools may assist with drafts, but the contributor remains responsible for understanding,
testing, and reviewing every submitted change. State the material assistance in the pull
request description when it affected code, tests, or documentation.

## Reporting concerns

Use GitHub Issues for bugs, feature requests, and development questions. Do not place
credentials, production endpoints, payloads, or sensitive security details in a public issue.
For a sensitive security report, follow the private reporting process in
[SECURITY.md](SECURITY.md). Community standards and conduct-reporting guidance are in
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Reference guidance

This guide adapts contribution practices from GitHub's
[contributor-guidelines guidance](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors)
to ModernLink's compatibility and verification constraints. It does not adopt their
project-specific contributor agreement or review policy.
