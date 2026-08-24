# Security Policy

## Supported versions

Security fixes are assessed for the current `main` branch and the most recent released version
of ModernLink. Older versions may receive guidance, but are not guaranteed to receive patches.

## Reporting a vulnerability

Do not report suspected vulnerabilities in a public GitHub issue. Send a private report to
[dyam.marcano@gmail.com](mailto:dyam.marcano@gmail.com) with the subject
`[ModernLink security]`.

Include, where safely possible:

- affected version, commit, or component;
- reproduction steps or a minimal proof of concept;
- impact and any prerequisites;
- suggested mitigations, if known.

Do not include credentials, real broker endpoints, customer code, message payloads, or other
sensitive production data. Redact them before sending the report.

## Handling process

The maintainer will acknowledge a report within five business days and will coordinate a fix or
explain why the report cannot be reproduced. Please allow time for investigation and a fix before
public disclosure. The reporter will be kept informed of material progress when contact details
are provided.

Security fixes may be released before a detailed public advisory. Once remediation is available,
the project may publish affected versions, impact, mitigations, and appropriate credit to the
reporter.

## Security boundaries

ModernLink handles native JNI code, TLS, messaging transports, binary distribution, and legacy
Java integration. Reports involving credential exposure, unsafe native loading, TLS validation,
message delivery guarantees, deserialization, or dependency compromise are particularly useful.
