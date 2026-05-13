"""Execute the Python evidence workflow guide's checked example.

This keeps docs/guides/python-evidence-workflow.md tied to the installed hl7v2
wheel instead of letting the long guide script drift from the binding API.
"""

from __future__ import annotations

import contextlib
import io
import json
import re
import sys
from pathlib import Path


GUIDE_PATH = Path("docs/guides/python-evidence-workflow.md")
SCRIPT_MARKER = 'ROOT = Path("target/hl7v2-python-evidence")'
SCHEMA_DIR = Path("schemas/evidence")
ARTIFACT_SCHEMAS = {
    "validation-report-v2.json": "validation-report-v2.schema.json",
    "corpus-summary-v2.json": "corpus-summary-v2.schema.json",
    "corpus-fingerprint-v2.json": "corpus-fingerprint-v2.schema.json",
    "corpus-diff-v2.json": "corpus-diff-v2.schema.json",
    "redaction-output-v2.json": "safe-analysis-redaction-output-v2.schema.json",
    "bundle-summary-v2.json": "evidence-bundle-v2.schema.json",
    "replay-report-v2.json": "evidence-replay-v2.schema.json",
}
BUNDLE_ARTIFACT_SCHEMAS = {
    "manifest.json": "evidence-bundle-manifest-v2.schema.json",
    "environment.json": "evidence-bundle-environment-v2.schema.json",
    "field-paths.json": "field-path-trace-v2.schema.json",
    "redaction-receipt.json": "redaction-receipt-v2.schema.json",
    "validation-report.json": "validation-report-v2.schema.json",
}


def extract_workflow_script() -> str:
    """Extract the guide's end-to-end Python workflow block."""
    guide = GUIDE_PATH.read_text(encoding="utf-8")
    for block in re.findall(r"```python\n(.*?)\n```", guide, flags=re.DOTALL):
        if SCRIPT_MARKER in block:
            return block
    raise RuntimeError(f"could not find Python workflow block in {GUIDE_PATH}")


class SchemaValidationError(ValueError):
    """Raised when a generated evidence artifact fails its checked schema."""


def resolve_ref(root_schema: dict, ref: str) -> dict:
    """Resolve a local JSON Schema reference."""
    if not ref.startswith("#/"):
        raise SchemaValidationError(f"unsupported non-local schema ref {ref}")

    current: object = root_schema
    for part in ref[2:].split("/"):
        if not isinstance(current, dict) or part not in current:
            raise SchemaValidationError(f"schema ref {ref} could not resolve {part}")
        current = current[part]
    if not isinstance(current, dict):
        raise SchemaValidationError(f"schema ref {ref} did not resolve to an object")
    return current


def instance_matches_type(instance: object, expected_type: str) -> bool:
    """Return whether a JSON value matches a schema primitive type."""
    if expected_type == "object":
        return isinstance(instance, dict)
    if expected_type == "array":
        return isinstance(instance, list)
    if expected_type == "string":
        return isinstance(instance, str)
    if expected_type == "boolean":
        return isinstance(instance, bool)
    if expected_type == "integer":
        return isinstance(instance, int) and not isinstance(instance, bool)
    if expected_type == "number":
        return isinstance(instance, (int, float)) and not isinstance(instance, bool)
    if expected_type == "null":
        return instance is None
    raise SchemaValidationError(f"unsupported schema type {expected_type}")


def validate_schema(instance: object, schema: dict, root_schema: dict, path: str = "$") -> None:
    """Validate the schema subset used by checked evidence artifacts."""
    if "$ref" in schema:
        validate_schema(instance, resolve_ref(root_schema, schema["$ref"]), root_schema, path)
        return

    if "anyOf" in schema:
        for option in schema["anyOf"]:
            try:
                validate_schema(instance, option, root_schema, path)
                return
            except SchemaValidationError:
                continue
        raise SchemaValidationError(f"{path} did not match any allowed schema")

    if "oneOf" in schema:
        matches = 0
        for option in schema["oneOf"]:
            try:
                validate_schema(instance, option, root_schema, path)
            except SchemaValidationError:
                continue
            matches += 1
        if matches != 1:
            raise SchemaValidationError(f"{path} matched {matches} oneOf schemas")
        return

    if "const" in schema and instance != schema["const"]:
        raise SchemaValidationError(
            f"{path} expected const {schema['const']!r}, got {instance!r}"
        )

    if "enum" in schema and instance not in schema["enum"]:
        raise SchemaValidationError(f"{path} expected one of {schema['enum']!r}")

    expected = schema.get("type")
    if expected is not None:
        expected_types = expected if isinstance(expected, list) else [expected]
        if not any(instance_matches_type(instance, typ) for typ in expected_types):
            raise SchemaValidationError(
                f"{path} expected type {expected_types!r}, got {type(instance).__name__}"
            )

    if isinstance(instance, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in instance:
                raise SchemaValidationError(f"{path}.{key} is required")

        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            extra = sorted(set(instance) - set(properties))
            if extra:
                raise SchemaValidationError(f"{path} has unexpected properties {extra!r}")

        for key, value in instance.items():
            if key in properties:
                validate_schema(value, properties[key], root_schema, f"{path}.{key}")

    if isinstance(instance, list) and "items" in schema:
        if "minItems" in schema and len(instance) < schema["minItems"]:
            raise SchemaValidationError(f"{path} has fewer than {schema['minItems']} items")
        for index, item in enumerate(instance):
            validate_schema(item, schema["items"], root_schema, f"{path}[{index}]")

    if "minimum" in schema and isinstance(instance, (int, float)) and instance < schema["minimum"]:
        raise SchemaValidationError(f"{path} is below minimum {schema['minimum']}")

    if "minLength" in schema and isinstance(instance, str) and len(instance) < schema["minLength"]:
        raise SchemaValidationError(f"{path} is shorter than minLength {schema['minLength']}")

    if "pattern" in schema and isinstance(instance, str):
        if re.search(schema["pattern"], instance) is None:
            raise SchemaValidationError(f"{path} does not match pattern {schema['pattern']!r}")


def validate_artifact_against_schema(artifact_path: Path, schema_name: str) -> None:
    """Validate a generated JSON artifact against a checked-in schema."""
    schema_path = SCHEMA_DIR / schema_name
    artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    validate_schema(artifact, schema, schema)


def main() -> int:
    script = extract_workflow_script()
    stdout = io.StringIO()

    namespace = {
        "__builtins__": __builtins__,
        "__file__": str(GUIDE_PATH),
        "__name__": "__main__",
    }
    with contextlib.redirect_stdout(stdout):
        exec(compile(script, str(GUIDE_PATH), "exec"), namespace)

    raw_output = stdout.getvalue().strip()
    try:
        summary = json.loads(raw_output)
    except json.JSONDecodeError as error:
        print(f"guide workflow did not emit JSON: {raw_output}", file=sys.stderr)
        raise SystemExit(1) from error

    expected = {
        "after_message_count": 1,
        "bundle_artifacts": 10,
        "diff_field_presence_deltas": 0,
        "redaction_phi_removed": True,
        "replay_reproduced": True,
        "validation_issue_codes": ["value_not_in_set"],
        "validation_valid": False,
    }

    mismatches = {
        key: {"expected": value, "actual": summary.get(key)}
        for key, value in expected.items()
        if summary.get(key) != value
    }
    if mismatches:
        print(
            "guide workflow summary mismatches: "
            + json.dumps(mismatches, sort_keys=True),
            file=sys.stderr,
        )
        return 1

    version = summary.get("version")
    if not isinstance(version, str) or not version:
        print(f"guide workflow did not report an installed version: {summary}", file=sys.stderr)
        return 1

    reports_dir = Path("target/hl7v2-python-evidence/reports")
    for artifact, schema in ARTIFACT_SCHEMAS.items():
        artifact_path = reports_dir / artifact
        if not artifact_path.is_file():
            print(f"guide workflow did not write {artifact}", file=sys.stderr)
            return 1
        try:
            validate_artifact_against_schema(artifact_path, schema)
        except (json.JSONDecodeError, SchemaValidationError) as error:
            print(
                f"guide workflow artifact {artifact} failed {schema}: {error}",
                file=sys.stderr,
            )
            return 1

    bundle_dir = Path("target/hl7v2-python-evidence/issue-bundle")
    for artifact, schema in BUNDLE_ARTIFACT_SCHEMAS.items():
        artifact_path = bundle_dir / artifact
        if not artifact_path.is_file():
            print(f"guide workflow did not write bundle artifact {artifact}", file=sys.stderr)
            return 1
        try:
            validate_artifact_against_schema(artifact_path, schema)
        except (json.JSONDecodeError, SchemaValidationError) as error:
            print(
                f"guide workflow bundle artifact {artifact} failed {schema}: {error}",
                file=sys.stderr,
            )
            return 1

    print(f"python evidence workflow guide ok version={version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
