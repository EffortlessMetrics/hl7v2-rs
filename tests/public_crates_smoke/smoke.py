#!/usr/bin/env python3
"""Install public crates from crates.io and run first-use smoke checks."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
import uuid


def cargo(toolchain: str) -> list[str]:
    return ["cargo", f"+{toolchain}"]


def run(
    command: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    print("+ " + " ".join(command))
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        if result.stdout:
            print(result.stdout)
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        raise RuntimeError(f"command failed with exit code {result.returncode}: {command}")
    return result


def choose_scratch_parent() -> Path:
    override = os.environ.get("HL7V2_PUBLIC_CRATES_SMOKE_ROOT")
    if override:
        return Path(override)
    f_cargo = Path("F:/cargo-target")
    if f_cargo.exists():
        return f_cargo
    return Path(tempfile.gettempdir())


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def bin_path(root: Path, name: str) -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    return root / "bin" / f"{name}{suffix}"


def write_rust_smoke_main(path: Path) -> None:
    path.write_text(
        r'''fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = r#"
message_structure: "GENERIC"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;

    let hl7 = b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605160101||ADT^A01|CTRL1|P|2.5\rPID|1||MRN-1^^^HOSP^MR||Example^Patient";
    let message = hl7v2::parse(hl7)?;
    let profile = hl7v2::load_profile_checked(profile_yaml)?;
    let report =
        hl7v2::ValidationReport::from_issues(&message, Some("inline".to_string()), hl7v2::validate(&message, &profile));
    let normalized = hl7v2::normalize(hl7, true)?;
    let ack = hl7v2::ack(&message, hl7v2::AckCode::AA)?;
    let ack_text = String::from_utf8(hl7v2::write(&ack))?;

    assert!(report.valid, "validation report should pass");
    assert_eq!(report.message_type, "ADT^A01");
    assert!(normalized.ends_with(b"\r"), "normalized message should end with carriage return");
    assert!(ack_text.contains("MSA|AA|CTRL1"), "ACK should accept the source control ID");

    let receipt = serde_json::json!({
        "valid": report.valid,
        "message_type": report.message_type,
        "issue_count": report.issue_count,
        "normalized_bytes": normalized.len(),
        "ack": "AA",
    });
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
''',
        encoding="utf-8",
    )


def wait_for_ready(url: str, timeout_seconds: float = 20.0) -> dict[str, object]:
    deadline = time.monotonic() + timeout_seconds
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=2.0) as response:
                if response.status == 200:
                    return json.loads(response.read().decode("utf-8"))
        except Exception as exc:  # pragma: no cover - diagnostic path
            last_error = exc
        time.sleep(0.25)
    raise RuntimeError(f"server did not become ready at {url}: {last_error}")


def rust_library_smoke(scratch: Path, env: dict[str, str], toolchain: str, version: str) -> None:
    project = scratch / "rust-smoke"
    run(cargo(toolchain) + ["new", "rust-smoke", "--bin"], cwd=scratch, env=env)
    run(cargo(toolchain) + ["add", f"hl7v2@{version}"], cwd=project, env=env)
    run(cargo(toolchain) + ["add", "serde_json"], cwd=project, env=env)
    write_rust_smoke_main(project / "src" / "main.rs")
    result = run(cargo(toolchain) + ["run", "--quiet"], cwd=project, env=env)
    receipt = json.loads(result.stdout)
    if receipt["valid"] is not True or receipt["ack"] != "AA":
        raise RuntimeError(f"unexpected Rust first-use receipt: {receipt}")


def cli_smoke(scratch: Path, env: dict[str, str], toolchain: str, version: str) -> None:
    root = scratch / "cli-install"
    run(
        cargo(toolchain)
        + ["install", "hl7v2-cli", "--version", version, "--root", str(root)],
        env=env,
    )
    result = run([str(bin_path(root, "hl7v2-cli")), "doctor", "--format", "json"], env=env)
    report = json.loads(result.stdout)
    if report.get("version") != version:
        raise RuntimeError(f"unexpected CLI version in doctor report: {report.get('version')}")


def server_smoke(scratch: Path, env: dict[str, str], toolchain: str, version: str) -> None:
    root = scratch / "server-install"
    run(
        cargo(toolchain)
        + ["install", "hl7v2-server", "--version", version, "--root", str(root)],
        env=env,
    )
    server = bin_path(root, "hl7v2-server")
    config = run([str(server), "--print-config"], env=env)
    if "bind_address" not in config.stdout:
        raise RuntimeError("server --print-config did not include bind_address")

    port = free_port()
    server_env = env.copy()
    server_env["BIND_ADDRESS"] = f"127.0.0.1:{port}"
    process = subprocess.Popen(
        [str(server)],
        env=server_env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        ready = wait_for_ready(f"http://127.0.0.1:{port}/ready")
        if ready.get("version") != version:
            raise RuntimeError(f"unexpected server /ready version: {ready}")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", default="1.5.0")
    parser.add_argument("--toolchain", default="1.95.0")
    parser.add_argument("--keep", action="store_true", help="Keep the scratch directory")
    args = parser.parse_args()

    scratch_parent = choose_scratch_parent()
    scratch_parent.mkdir(parents=True, exist_ok=True)
    scratch = scratch_parent / f"hl7v2-rs-public-crates-smoke-{uuid.uuid4().hex[:8]}"
    scratch.mkdir()

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(scratch / "target")
    env.setdefault("CARGO_INCREMENTAL", "0")

    print(f"scratch={scratch}")
    try:
        rust_library_smoke(scratch, env, args.toolchain, args.version)
        cli_smoke(scratch, env, args.toolchain, args.version)
        server_smoke(scratch, env, args.toolchain, args.version)
        print(
            json.dumps(
                {
                    "version": args.version,
                    "rust_library": "pass",
                    "cli": "pass",
                    "server": "pass",
                    "python_registry": "not tested",
                    "npm_registry": "not tested",
                },
                indent=2,
                sort_keys=True,
            )
        )
    finally:
        if args.keep:
            print(f"kept scratch={scratch}")
        else:
            shutil.rmtree(scratch, ignore_errors=True)
            print(f"removed scratch={scratch}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
