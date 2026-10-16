#!/usr/bin/env bash
# Generate the release SBOM (SPDX + CycloneDX) from the source tree and check
# the NTIA minimum elements.
#
# Usage: .github/scripts/ci/generate-sbom.sh <version> <out_dir>
# Needs cargo-cyclonedx, syft and jq on PATH. Run from the repo root.
set -euo pipefail

version="${1:?usage: generate-sbom.sh <version> <out_dir>}"
out_dir="${2:?}"
supplier="IntellectusCorp"
spdx="$out_dir/int2dds-$version.spdx.json"
cdx="$out_dir/int2dds-$version.cdx.json"

# Syft would otherwise pick up a previous run's output as an input SBOM.
rm -rf "$out_dir"

# Syft's Cargo.lock cataloger records no licenses, so the Rust graph comes from
# cargo-cyclonedx (one SBOM per crate, all target platforms, no dev-dependencies)
# and Syft's sbom-cataloger merges those per-crate files.
tmp="$(mktemp -d)"
cargo cyclonedx --format json --spec-version 1.5 --target all
syft scan dir:. --quiet \
  --source-name int2dds --source-version "$version" \
  --select-catalogers "+sbom-cataloger,-rust-cargo-lock-cataloger,-github-actions-usage-cataloger,-github-action-workflow-usage-cataloger" \
  --exclude './java/gradle/**' \
  -o "spdx-json=$tmp/spdx.json" -o "cyclonedx-json=$tmp/cdx.json"
find . -name '*.cdx.json' -not -path './target/*' -delete
mkdir -p "$out_dir"

# --source-supplier would stamp every third-party package too; set it on the root only.
jq --arg s "$supplier" '.metadata.supplier = {name: $s} | .metadata.component.supplier = {name: $s}' \
  "$tmp/cdx.json" > "$cdx"
jq --arg s "Organization: $supplier" '
  [.relationships[] | select(.relationshipType == "DESCRIBES") | .relatedSpdxElement] as $roots
  | .packages |= map(if .SPDXID | IN($roots[]) then .supplier = $s else . end)' \
  "$tmp/spdx.json" > "$spdx"
rm -rf "$tmp"

# NTIA minimum elements: supplier, component name, version, unique identifier,
# dependency relationship, author, timestamp.
jq -e --arg v "$version" '
  .bomFormat == "CycloneDX"
  and .metadata.timestamp
  and (.metadata.tools.components | length > 0)
  and .metadata.supplier.name
  and .metadata.component.name == "int2dds" and .metadata.component.version == $v
  and (.dependencies | length > 0)
  and ([.components[] | select(.type == "library")] | length > 0)
  and ([.components[] | select(.type == "library" and ((.name and .version and .purl) | not))] | length == 0)
' "$cdx" > /dev/null || { echo "ERROR: $cdx is missing NTIA minimum elements" >&2; exit 1; }
jq -e '
  (.spdxVersion | startswith("SPDX-"))
  and .creationInfo.created
  and (.creationInfo.creators | length > 0)
  and (.packages | length > 1)
  and ([.relationships[] | select(.relationshipType == "DESCRIBES")] | length > 0)
  and ([.relationships[] | select(.relationshipType == "DEPENDENCY_OF" or .relationshipType == "DEPENDS_ON")] | length > 0)
' "$spdx" > /dev/null || { echo "ERROR: $spdx is not a valid SPDX document" >&2; exit 1; }

jq -r '
  [.components[] | select(.type == "library")] as $libs
  | "components: \($libs | length) (license declared: \([$libs[] | select(.licenses)] | length))"
' "$cdx"
ls -1 "$out_dir"
