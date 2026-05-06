# Release artifact verification

Every release of `repo-trust` ships with two independent integrity
controls attached to each archive:

1. **SLSA provenance attestations** — emitted by GitHub's
   `actions/attest-build-provenance`. These prove the archive was
   produced by this repository's `release.yml` workflow at a specific
   commit.
2. **Sigstore keyless signatures** — emitted by `cosign sign-blob`
   using GitHub Actions OIDC. These provide a detached signature
   (`.sig`) and a short-lived certificate (`.pem`) that bind the
   archive to the workflow identity that signed it.

Both controls are non-overlapping: provenance proves *how* and *from
where* the artifact was built; cosign proves *which workflow run*
signed it. Verifying both gives a stronger guarantee than either
alone.

## Files attached to each release

For a target like `x86_64-unknown-linux-gnu` you will find:

| File                                                  | Purpose                            |
|-------------------------------------------------------|------------------------------------|
| `repo-trust-x86_64-unknown-linux-gnu.tar.gz`          | The release archive                |
| `repo-trust-x86_64-unknown-linux-gnu.tar.gz.sha256`   | SHA-256 checksum                   |
| `repo-trust-x86_64-unknown-linux-gnu.tar.gz.sig`      | Cosign detached signature          |
| `repo-trust-x86_64-unknown-linux-gnu.tar.gz.pem`      | Sigstore short-lived certificate   |

Windows targets ship the same set with `.zip` instead of `.tar.gz`.

## Verifying a release archive

You will need the [`cosign`](https://docs.sigstore.dev/system_config/installation/)
CLI (v2.0+).

Download the archive, signature, and certificate from the release
page, then run:

```bash
cosign verify-blob \
  --certificate repo-trust-<target>.tar.gz.pem \
  --signature repo-trust-<target>.tar.gz.sig \
  --certificate-identity-regexp 'https://github.com/Dmitrze/repo-trust' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  repo-trust-<target>.tar.gz
```

Expected output:

```
Verified OK
```

The `--certificate-identity-regexp` flag pins the signing identity to
this repository's GitHub Actions workflow. The `--certificate-oidc-issuer`
flag pins the trust root to GitHub's OIDC provider. Both must match
for verification to succeed; either one diverging means the artifact
was not produced by this repository's release pipeline.

## Verifying SLSA provenance (optional, additional control)

```bash
gh attestation verify repo-trust-<target>.tar.gz \
  --repo Dmitrze/repo-trust
```

This is independent of the cosign signature and uses GitHub's native
attestation store.

## Why these controls exist

`repo-trust` analyses other people's repositories for supply-chain
trustworthiness. The project would be hypocritical if its own release
pipeline were unverifiable. SLSA + Sigstore is the same combination
the OpenSSF Scorecard's `Signed-Releases` and `SBOM` checks recognise,
and it is what `repo-trust` itself looks for when scoring a target
repo's `Release Pipeline` factor.
