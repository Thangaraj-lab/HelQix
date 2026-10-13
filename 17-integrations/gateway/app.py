"""Local gateway for HELQIX engineering validation and tool integrations.

The gateway never fabricates simulation results. Engineering model validation
is reported separately from external simulation and only reports actual tool
outputs when a configured toolchain completes.
"""
from __future__ import annotations

import asyncio
import hashlib
import hmac
import ipaddress
import json
import math
import os
import re
import secrets
import sqlite3
import shutil
import sys
import tempfile
import threading
import time
import uuid
from pathlib import Path
from typing import Any

from fastapi import Cookie, FastAPI, Header, HTTPException, Query, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, ConfigDict, Field, ValidationError

ROOT = Path(os.environ.get("HELQIX_WORKSPACE_ROOT", Path(__file__).resolve().parents[2])).resolve()
INTELLIGENCE_ROOT = ROOT / "19-intelligence" / "python"
if str(INTELLIGENCE_ROOT) not in sys.path:
    sys.path.insert(0, str(INTELLIGENCE_ROOT))
from helqix_intelligence import ai_review_provider_status, parse_engineering_intent, review_engineering_intent_with_ai
DOMAIN_ROOT = ROOT / "18-libraries" / "python"
if str(DOMAIN_ROOT) not in sys.path:
    sys.path.insert(0, str(DOMAIN_ROOT))
from helqix_drone_domain import EngineeringBranch, EngineeringReality, ExplicitVerificationCheck, component_type_catalog, validate_resource_payload
from helqix_drone_domain.models import EVBatteryVehicleConfiguration
from helqix_drone_domain import DroneSpecification
from proofloop import recommend_next_step

API_KEY = os.environ.get("HELQIX_INTEGRATION_API_KEY", "")
RUNS_DB = Path(os.environ.get("HELQIX_INTEGRATION_RUNS_DB", str(ROOT / "data" / "integration_runs.sqlite3"))).resolve()
PX4_DIR = Path(os.environ["HELQIX_PX4_DIR"]).resolve() if os.environ.get("HELQIX_PX4_DIR") else None
FREECAD_CMD = os.environ.get("HELQIX_FREECAD_CMD", "FreeCADCmd")
BLENDER_CMD = os.environ.get("HELQIX_BLENDER_CMD", "blender")
PYTHON_RUNTIME = os.environ.get("PYTHON") or sys.executable
FOAM_SOLVERS = {"simpleFoam", "pimpleFoam", "buoyantPimpleFoam", "foamRun"}
PX4_MODELS = {"gz_x500", "gz_x500_depth", "gz_x500_vision", "gz_standard_vtol", "gz_rc_cessna"}

app = FastAPI(title="HELQIX Integration Gateway", version="0.1.0", docs_url=None, redoc_url=None)
app.add_middleware(
    CORSMiddleware,
    allow_origins=[x.strip() for x in os.environ.get("HELQIX_ALLOWED_ORIGINS", ",".join(f"http://{host}:{port}" for host in ("127.0.0.1", "localhost") for port in range(5173, 5179))).split(",") if x.strip()],
    allow_methods=["GET", "POST", "PUT", "DELETE"],
    allow_headers=["Authorization", "Content-Type", "If-Match", "Idempotency-Key"],
    expose_headers=["ETag"],
    allow_credentials=True,
)


class Operation(BaseModel):
    model_config = ConfigDict(extra="forbid")
    operation: str
    inputs: dict[str, Any] = Field(default_factory=dict)


class SignUpRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    name: str = Field(min_length=1, max_length=100)
    email: str = Field(min_length=3, max_length=254)
    password: str = Field(min_length=12, max_length=256)
    organization: str = Field(default="", max_length=120)


class LoginRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    email: str = Field(min_length=3, max_length=254)
    password: str = Field(min_length=1, max_length=256)


class ResourceCreate(BaseModel):
    model_config = ConfigDict(extra="forbid")
    payload: dict[str, Any]
    id: str | None = None
    project_id: str | None = None


class ResourceReplace(BaseModel):
    model_config = ConfigDict(extra="forbid")
    payload: dict[str, Any]
    project_id: str | None = None


class IntentParseRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    intent: str = Field(min_length=1, max_length=5000)
    domain: str | None = Field(default=None, max_length=120)
    assisted: bool = False


class ProofLoopRecordRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    action_id: str = Field(min_length=1, max_length=100)


ALLOWED_RESOURCES = {"projects", "realities", "executions", "branches", "evidence", "verifications", "failures", "causal", "searches", "states", "timelines", "proofloops"}
PASSWORD_ITERATIONS = 310_000
COOKIE_NAME = "helqix_session"
COOKIE_SECURE = os.environ.get("HELQIX_COOKIE_SECURE", "false").lower() == "true"


class SitlSession:
    def __init__(self, process: asyncio.subprocess.Process, model: str, user_id: str | None = None):
        self.id = str(uuid.uuid4())
        self.model = model
        self.process = process
        self.user_id = user_id
        self.started_at = time.time()

    def as_dict(self) -> dict[str, Any]:
        return {"id": self.id, "model": self.model, "pid": self.process.pid,
                "status": "running" if self.process.returncode is None else "stopped",
                "exit_code": self.process.returncode, "started_at": self.started_at}


sessions: dict[str, SitlSession] = {}
sessions_lock = asyncio.Lock()
_schema_lock = threading.Lock()
_local_workspace_lock = threading.Lock()
_schema_initialized = False
_pybamm_readiness: tuple[bool, str | None] | None = None
_pybamm_lock = asyncio.Lock()
_pybamm_check_task: asyncio.Task[tuple[bool, str | None]] | None = None


class ClosingSQLiteConnection(sqlite3.Connection):
    """Keep the gateway's existing `with open_database()` contract leak-free."""
    def __exit__(self, exc_type: Any, exc_value: Any, traceback: Any) -> bool | None:
        try:
            return super().__exit__(exc_type, exc_value, traceback)
        finally:
            self.close()


def authorize(authorization: str | None, session_token: str | None = None) -> None:
    if API_KEY:
        if authorization != f"Bearer {API_KEY}":
            raise HTTPException(status_code=401, detail="A valid HELQIX integration API key is required")
    else:
        get_current_user(session_token)


def open_database() -> sqlite3.Connection:
    global _schema_initialized
    RUNS_DB.parent.mkdir(parents=True, exist_ok=True)
    database = sqlite3.connect(RUNS_DB, timeout=15, factory=ClosingSQLiteConnection)
    database.row_factory = sqlite3.Row
    database.execute("PRAGMA foreign_keys=ON")
    database.execute("PRAGMA busy_timeout=15000")
    if not _schema_initialized:
        with _schema_lock:
            if not _schema_initialized:
                database.executescript("""
      CREATE TABLE IF NOT EXISTS helqix_users (
        id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE,
        organization TEXT NOT NULL DEFAULT '', password_salt BLOB NOT NULL,
        password_hash BLOB NOT NULL, created_at REAL NOT NULL
      );
      CREATE TABLE IF NOT EXISTS helqix_sessions (
        token_hash TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES helqix_users(id) ON DELETE CASCADE,
        expires_at REAL NOT NULL, created_at REAL NOT NULL
      );
      CREATE TABLE IF NOT EXISTS helqix_resources (
        id TEXT PRIMARY KEY, resource_type TEXT NOT NULL,
        user_id TEXT NOT NULL REFERENCES helqix_users(id) ON DELETE CASCADE,
        project_id TEXT, payload_json TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 1,
        created_at REAL NOT NULL, updated_at REAL NOT NULL
      );
      CREATE INDEX IF NOT EXISTS helqix_resources_owner_idx ON helqix_resources(user_id, resource_type, created_at DESC);
      CREATE TABLE IF NOT EXISTS helqix_idempotency (
        user_id TEXT NOT NULL, resource_type TEXT NOT NULL, request_key TEXT NOT NULL,
        fingerprint TEXT NOT NULL, resource_id TEXT NOT NULL, created_at REAL NOT NULL,
        PRIMARY KEY(user_id, resource_type, request_key)
      );
      CREATE TABLE IF NOT EXISTS integration_runs (
        id TEXT PRIMARY KEY, created_at REAL NOT NULL, tool TEXT NOT NULL, operation TEXT NOT NULL,
        inputs_json TEXT NOT NULL, result_json TEXT NOT NULL, provenance TEXT NOT NULL
      );
      CREATE TABLE IF NOT EXISTS helqix_integration_runs (
        id TEXT PRIMARY KEY, user_id TEXT, created_at REAL NOT NULL, tool TEXT NOT NULL, operation TEXT NOT NULL,
        inputs_json TEXT NOT NULL, result_json TEXT NOT NULL, provenance TEXT NOT NULL, project_id TEXT, project_snapshot_json TEXT
      );
      CREATE TABLE IF NOT EXISTS helqix_audit_events (
        seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE, created_at REAL NOT NULL,
        actor_user_id TEXT NOT NULL, project_id TEXT, action TEXT NOT NULL,
        subject_type TEXT NOT NULL, subject_id TEXT, outcome TEXT NOT NULL DEFAULT 'recorded',
        safe_details_json TEXT NOT NULL DEFAULT '{}'
      );
      CREATE INDEX IF NOT EXISTS helqix_audit_project_idx ON helqix_audit_events(project_id,seq);
      CREATE TRIGGER IF NOT EXISTS helqix_audit_no_update BEFORE UPDATE ON helqix_audit_events
        BEGIN SELECT RAISE(ABORT,'audit events are append-only'); END;
      CREATE TRIGGER IF NOT EXISTS helqix_audit_no_delete BEFORE DELETE ON helqix_audit_events
        BEGIN SELECT RAISE(ABORT,'audit events are append-only'); END;
    """)
                columns = {row["name"] for row in database.execute("PRAGMA table_info(helqix_integration_runs)")}
                if "project_id" not in columns:
                    database.execute("ALTER TABLE helqix_integration_runs ADD COLUMN project_id TEXT")
                if "project_snapshot_json" not in columns:
                    database.execute("ALTER TABLE helqix_integration_runs ADD COLUMN project_snapshot_json TEXT")
                database.execute("CREATE INDEX IF NOT EXISTS helqix_runs_project_idx ON helqix_integration_runs(user_id, project_id, created_at DESC)")
                _schema_initialized = True
    return database


def user_public(row: sqlite3.Row) -> dict[str, str]:
    return {"id": row["id"], "name": row["name"], "email": row["email"], "org": row["organization"]}


def get_current_user(session_token: str | None) -> sqlite3.Row:
    if not session_token or len(session_token) > 200:
        raise HTTPException(status_code=401, detail="Please sign in to continue")
    digest = hashlib.sha256(session_token.encode("utf-8")).hexdigest()
    with open_database() as database:
        row = database.execute("SELECT u.* FROM helqix_sessions s JOIN helqix_users u ON u.id=s.user_id WHERE s.token_hash=? AND s.expires_at>?", (digest, time.time())).fetchone()
    if row is None:
        raise HTTPException(status_code=401, detail="Your session expired; please sign in again")
    return row


def validate_resource(resource: str) -> str:
    if resource not in ALLOWED_RESOURCES:
        raise HTTPException(status_code=404, detail="Unknown HELQIX resource")
    return resource


def resource_record(row: sqlite3.Row) -> dict[str, Any]:
    return {"id": row["id"], "kind": row["resource_type"], "project_id": row["project_id"],
        "version": row["version"], "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(row["created_at"])),
        "updated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(row["updated_at"])), "payload": json.loads(row["payload_json"])}


def serialize_payload(payload: dict[str, Any]) -> str:
    try:
        return json.dumps(payload, separators=(",", ":"), allow_nan=False)
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=422, detail="Payload must contain only finite JSON values") from exc


def validate_domain_payload(resource: str, payload: dict[str, Any]) -> dict[str, Any]:
    try:
        return validate_resource_payload(resource, payload)
    except ValidationError as exc:
        issues = "; ".join(f"{'/'.join(str(part) for part in issue['loc'])}: {issue['msg']}" for issue in exc.errors())
        raise HTTPException(status_code=422, detail=f"Invalid {resource} design data: {issues}") from exc


def executable(name: str) -> str | None:
    found = shutil.which(name)
    if found:
        return found
    candidate = Path(name)
    if candidate.is_file():
        return str(candidate.resolve())
    return None


def discovered_program(configured: str, names: list[str], patterns: list[str]) -> str | None:
    direct = executable(configured)
    if direct:
        return direct
    for name in names:
        direct = executable(name)
        if direct:
            return direct
    roots = [Path(os.environ.get("ProgramFiles", r"C:\Program Files")), Path(os.environ.get("LOCALAPPDATA", "")) / "Programs"]
    for root in roots:
        for pattern in patterns:
            for candidate in sorted(root.glob(pattern), reverse=True):
                if candidate.is_file():
                    return str(candidate.resolve())
    return None


def freecad_program() -> str | None:
    return discovered_program(FREECAD_CMD, ["FreeCADCmd.exe", "FreeCADCmd", "freecadcmd"], ["FreeCAD*/bin/FreeCADCmd.exe", "FreeCAD*/FreeCADCmd.exe"])


def blender_program() -> str | None:
    return discovered_program(BLENDER_CMD, ["blender.exe", "blender"], ["Blender Foundation/Blender */blender.exe", "Blender*/blender.exe"])


def tool_status(tool_id: str, label: str, commands: list[str], configured: bool = True, note: str = "") -> dict[str, Any]:
    found = {name: executable(name) for name in commands}
    available = configured and any(found.values())
    missing = [name for name, path in found.items() if not path]
    return {"id": tool_id, "label": label, "status": "ready" if available else "unavailable",
            "available": available, "executables": found, "missing": missing,
            "note": note if available else (note or "Install the tool and expose its executable on PATH.")}


async def inventory() -> list[dict[str, Any]]:
    global _pybamm_check_task
    if _pybamm_readiness is None and (_pybamm_check_task is None or _pybamm_check_task.done()):
        _pybamm_check_task = asyncio.create_task(pybamm_runtime())
    pybamm_ready = _pybamm_readiness is not None and _pybamm_readiness[0]
    pybamm_checking = _pybamm_readiness is None
    pybamm_problem = "Runtime check in progress" if pybamm_checking else (_pybamm_readiness[1] if _pybamm_readiness else None)
    return [
        {"id": "helqix_reality", "label": "HELQIX Reality Validator", "status": "ready", "available": True,
         "executables": {"runtime": "gateway"}, "missing": [],
         "note": "Checks the saved, domain-neutral reality structure and project references. This is structural validation, not physics or performance simulation."},
        {**tool_status("freecad", "FreeCAD", [freecad_program() or FREECAD_CMD], note="Parametric CAD and STEP inspection."), "executables": {"FreeCADCmd": freecad_program()}},
        {**tool_status("blender", "Blender", [blender_program() or BLENDER_CMD], note="Mesh asset inspection and visual export."), "executables": {"blender": blender_program()}},
        {"id": "pybamm", "label": "PyBaMM", "status": "ready" if pybamm_ready else ("checking" if pybamm_checking else "unavailable"),
         "available": pybamm_ready, "executables": {"python": PYTHON_RUNTIME},
         "missing": [] if pybamm_ready else [pybamm_problem or "Python package pybamm"],
         "note": "Physics-based single-cell SPM/Chen2020 discharge, plus a saved-project EV drive-cycle adapter that couples longitudinal load to identical series/parallel cells." if pybamm_ready else ("Checking the PyBaMM runtime; no results are available yet." if pybamm_checking else "The installed package failed a runtime import check; no battery results will be returned.")},
        tool_status("openfoam", "OpenFOAM", ["blockMesh", "simpleFoam", "pimpleFoam"], note="Mesh and CFD solver execution for existing OpenFOAM cases."),
        {"id": "px4_gazebo", "label": "PX4 SITL + Gazebo", "status": "ready" if PX4_DIR and (PX4_DIR / "Makefile").is_file() and executable("make") and executable("gz") else "unavailable",
         "available": bool(PX4_DIR and (PX4_DIR / "Makefile").is_file() and executable("make") and executable("gz")),
         "executables": {"make": executable("make"), "gazebo": executable("gz")},
         "missing": [name for name, ready in (("PX4 source checkout (HELQIX_PX4_DIR)", bool(PX4_DIR and (PX4_DIR / "Makefile").is_file())), ("make", bool(executable("make"))), ("Gazebo gz", bool(executable("gz")))) if not ready],
         "note": "PX4 software-in-the-loop flight simulation through modern Gazebo; multi-vehicle runs need Linux."},
    ]


async def pybamm_runtime() -> tuple[bool, str | None]:
    global _pybamm_readiness
    if _pybamm_readiness is not None:
        return _pybamm_readiness
    async with _pybamm_lock:
        if _pybamm_readiness is not None:
            return _pybamm_readiness
        try:
            process = await asyncio.create_subprocess_exec(PYTHON_RUNTIME, "-c", "import pybamm",
                stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.PIPE)
            _, stderr = await asyncio.wait_for(process.communicate(), timeout=20)
        except asyncio.TimeoutError:
            process.kill()
            await process.wait()
            _pybamm_readiness = (False, "PyBaMM runtime import exceeded the 20 second health check")
        except OSError as exc:
            _pybamm_readiness = (False, f"PyBaMM runtime could not start: {type(exc).__name__}")
        else:
            if process.returncode == 0:
                _pybamm_readiness = (True, None)
            else:
                detail = stderr.decode("utf-8", errors="replace")[-500:].strip().splitlines()
                _pybamm_readiness = (False, f"PyBaMM runtime failed: {detail[-1][:220] if detail else 'import failed'}")
        return _pybamm_readiness


def persist_run(tool: str, operation: str, inputs: dict[str, Any], result: dict[str, Any], provenance: str,
                user_id: str | None, project_id: str | None = None,
                project_snapshot: dict[str, Any] | None = None) -> dict[str, Any]:
    run_id, created_at = str(uuid.uuid4()), time.time()
    RUNS_DB.parent.mkdir(parents=True, exist_ok=True)
    with open_database() as database:
        database.execute("INSERT INTO helqix_integration_runs (id,user_id,created_at,tool,operation,inputs_json,result_json,provenance,project_id,project_snapshot_json) VALUES (?,?,?,?,?,?,?,?,?,?)",
            (run_id, user_id, created_at, tool, operation, json.dumps(inputs, separators=(",", ":")),
             json.dumps(result, separators=(",", ":")), provenance, project_id,
             json.dumps(project_snapshot, separators=(",", ":")) if project_snapshot is not None else None))
        if user_id and project_id:
            _append_audit_event(database, user_id, project_id, "tool_run.recorded", "integration_run", run_id,
            "failed" if result.get("status") == "failed" else ("completed" if result.get("status") == "completed" else "recorded"),
                                {"tool": tool, "operation": operation, "provenance": provenance})
    return {"id": run_id, "created_at": created_at, "tool": tool, "operation": operation,
            "inputs": inputs, "result": result, "provenance": provenance, "project_id": project_id,
            "project_snapshot": project_snapshot}


def _append_audit_event(database: sqlite3.Connection, actor_user_id: str, project_id: str | None,
                        action: str, subject_type: str, subject_id: str | None,
                        outcome: str = "recorded", safe_details: dict[str, Any] | None = None) -> None:
    """Store a minimal event; callers must never pass user content, credentials, or tool payloads."""
    database.execute(
        "INSERT INTO helqix_audit_events (id,created_at,actor_user_id,project_id,action,subject_type,subject_id,outcome,safe_details_json) VALUES (?,?,?,?,?,?,?,?,?)",
        (str(uuid.uuid4()), time.time(), actor_user_id, project_id, action, subject_type, subject_id,
         outcome, json.dumps(safe_details or {}, sort_keys=True, separators=(",", ":"), allow_nan=False)))


def persist_project_resource(resource_type: str, payload: dict[str, Any], user_id: str | None,
                            project_id: str) -> dict[str, Any]:
    record_id = str(uuid.uuid4())
    now = time.time()
    with open_database() as database:
        database.execute("INSERT INTO helqix_resources (id,resource_type,user_id,project_id,payload_json,version,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?)",
                         (record_id, resource_type, user_id, project_id,
                          json.dumps(payload, separators=(",", ":"), allow_nan=False), now, now))
        row = database.execute("SELECT * FROM helqix_resources WHERE id=?", (record_id,)).fetchone()
    return resource_record(row)


def project_snapshot(project_row: sqlite3.Row) -> dict[str, Any]:
    """Capture hashes of declared project model inputs at the time a tool run starts."""
    payload = json.loads(project_row["payload_json"])
    model_digests: dict[str, Any] = {}
    for key in ("engineering_reality", "specification", "ev_vehicle"):
        value = payload.get(key)
        if not isinstance(value, dict):
            continue
        canonical = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
        model_digests[key] = {
            "schema": value.get("schema"),
            "revision": (value.get("revision", 1) if key == "engineering_reality" else project_row["version"]),
            "sha256": hashlib.sha256(canonical).hexdigest(),
        }
    return {
        "schema": "helqix.project-snapshot.v1",
        "project_id": project_row["id"],
        "project_version": project_row["version"],
        "captured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "models": model_digests,
    }


def owned_project(project_id: Any, user_id: str | None) -> sqlite3.Row:
    if user_id is None:
        raise HTTPException(status_code=401, detail="An authenticated account session is required for project-scoped access")
    if not isinstance(project_id, str):
        raise HTTPException(status_code=422, detail="A saved project_id is required for this operation")
    try:
        project_id = str(uuid.UUID(project_id))
    except ValueError as exc:
        raise HTTPException(status_code=422, detail="project_id must be a UUID") from exc
    with open_database() as database:
        row = database.execute("SELECT * FROM helqix_resources WHERE id=? AND resource_type='projects' AND user_id=?",
                               (project_id, user_id)).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="Project not found")
    return row


def workspace_file(raw: Any, *, suffixes: set[str] | None = None) -> Path:
    if not isinstance(raw, str) or not raw.strip():
        raise HTTPException(status_code=422, detail="A workspace-relative file path is required")
    candidate = (ROOT / raw).resolve()
    if not candidate.is_relative_to(ROOT) or not candidate.is_file():
        raise HTTPException(status_code=422, detail="File must exist inside the configured HELQIX workspace")
    if suffixes and candidate.suffix.lower() not in suffixes:
        raise HTTPException(status_code=422, detail=f"Supported file types: {', '.join(sorted(suffixes))}")
    return candidate


def workspace_output(raw: Any, *, suffix: str) -> Path:
    if not isinstance(raw, str) or not raw.strip():
        raise HTTPException(status_code=422, detail="A workspace-relative output path is required")
    candidate = (ROOT / raw).resolve()
    if not candidate.is_relative_to(ROOT) or candidate.suffix.lower() != suffix.lower():
        raise HTTPException(status_code=422, detail=f"Output must be a {suffix} file inside the configured HELQIX workspace")
    if candidate.exists():
        raise HTTPException(status_code=409, detail="Output file already exists; choose a new output path")
    candidate.parent.mkdir(parents=True, exist_ok=True)
    return candidate


async def _read_process_tail(stream: asyncio.StreamReader, limit: int = 20_000) -> bytes:
    """Drain a child pipe without allowing a noisy solver to consume unbounded memory."""
    tail = bytearray()
    while chunk := await stream.read(8192):
        tail.extend(chunk)
        if len(tail) > limit:
            del tail[:-limit]
    return bytes(tail)


async def run_process(args: list[str], *, cwd: Path | None = None, timeout: int = 120) -> dict[str, Any]:
    try:
        proc = await asyncio.create_subprocess_exec(*args, cwd=str(cwd) if cwd else None,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        stdout_task = asyncio.create_task(_read_process_tail(proc.stdout))
        stderr_task = asyncio.create_task(_read_process_tail(proc.stderr))
        exit_task = asyncio.create_task(proc.wait())
        try:
            stdout, stderr, _ = await asyncio.wait_for(
                asyncio.gather(stdout_task, stderr_task, exit_task), timeout=timeout)
        except asyncio.TimeoutError:
            if proc.returncode is None:
                try: proc.kill()
                except ProcessLookupError: pass
            await proc.wait()
            await asyncio.gather(stdout_task, stderr_task, exit_task, return_exceptions=True)
            raise
        except asyncio.CancelledError:
            if proc.returncode is None:
                try: proc.kill()
                except ProcessLookupError: pass
            await proc.wait()
            await asyncio.gather(stdout_task, stderr_task, exit_task, return_exceptions=True)
            raise
    except asyncio.TimeoutError:
        raise HTTPException(status_code=504, detail=f"Tool exceeded its {timeout}s operation timeout")
    except OSError as exc:
        raise HTTPException(status_code=503, detail=f"Could not start external tool: {exc}") from exc
    out = stdout.decode("utf-8", errors="replace")
    err = stderr.decode("utf-8", errors="replace")
    return {"ok": proc.returncode == 0, "exit_code": proc.returncode, "stdout": out, "stderr": err}


async def run_tool_process(*, tool: str, operation: str, inputs: dict[str, Any], provenance: str,
                           user_id: str | None, project_id: str | None,
                           project_snapshot: dict[str, Any] | None, args: list[str],
                           cwd: Path | None = None, timeout: int = 120,
                           failure_message: str) -> dict[str, Any]:
    """Execute an external tool and persist bounded output for every attempted run."""
    try:
        result = await run_process(args, cwd=cwd, timeout=timeout)
    except HTTPException as exc:
        run = persist_run(tool, operation, inputs, {
            "status": "failed", "failure_stage": "process_start_or_timeout",
            "http_status": exc.status_code, "error": str(exc.detail)[:1000],
            "stdout": "", "stderr": "",
        }, provenance, user_id, project_id, project_snapshot)
        raise HTTPException(status_code=exc.status_code,
                            detail={"message": failure_message, "run_id": run["id"]}) from exc
    if not result["ok"]:
        run = persist_run(tool, operation, inputs, {
            "status": "failed", "failure_stage": "external_process",
            "exit_code": result["exit_code"], "stdout": result["stdout"], "stderr": result["stderr"],
        }, provenance, user_id, project_id, project_snapshot)
        raise HTTPException(status_code=422,
                            detail={"message": failure_message, "run_id": run["id"]})
    return result


def decode_tool_report(result: dict[str, Any], *, tool: str, operation: str, inputs: dict[str, Any],
                       provenance: str, user_id: str | None, project_id: str | None,
                       project_snapshot: dict[str, Any] | None, failure_message: str) -> dict[str, Any]:
    """Parse the tool's bounded machine report, persisting malformed or absent reports as failures."""
    line = next((item[len("HELQIX_RESULT="):] for item in result["stdout"].splitlines()
                 if item.startswith("HELQIX_RESULT=")), None)
    try:
        if line is None:
            raise ValueError("Tool completed without its HELQIX_RESULT report marker.")
        payload = json.loads(line)
        if not isinstance(payload, dict):
            raise ValueError("Tool result report must be a JSON object.")
    except (json.JSONDecodeError, ValueError) as exc:
        run = persist_run(tool, operation, inputs, {
            "status": "failed", "failure_stage": "result_report",
            "error": str(exc)[:1000], "exit_code": result.get("exit_code"),
            "stdout": result.get("stdout", ""), "stderr": result.get("stderr", ""),
        }, provenance, user_id, project_id, project_snapshot)
        raise HTTPException(status_code=502,
                            detail={"message": failure_message, "run_id": run["id"]}) from exc
    return payload


@app.get("/health/live")
async def live() -> dict[str, str]:
    return {"status": "live"}


@app.get("/v1/integrations")
async def list_integrations(authorization: str | None = Header(default=None), session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    authorize(authorization, session_token)
    return {"integrations": await inventory()}


@app.get("/v1/integrations/runs")
async def list_runs(limit: int = 20, project_id: str | None = None,
                    authorization: str | None = Header(default=None),
                    session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    authorize(authorization, session_token)
    user = get_current_user(session_token) if session_token else None
    if not 1 <= limit <= 100:
        raise HTTPException(status_code=422, detail="limit must be between 1 and 100")
    user_id = user["id"] if user else None
    if project_id and user_id is None:
        raise HTTPException(status_code=401, detail="An authenticated account session is required for project-scoped run history")
    if not RUNS_DB.exists():
        return {"runs": []}
    if project_id:
        project = owned_project(project_id, user_id)
        project_id = project["id"]
    query = "SELECT * FROM helqix_integration_runs WHERE 1=1"
    values: list[Any] = []
    if user_id is None:
        query += " AND user_id IS NULL"
    else:
        query += " AND user_id=?"; values.append(user_id)
    if project_id:
        query += " AND project_id=?"; values.append(project_id)
    query += " ORDER BY created_at DESC LIMIT ?"; values.append(limit)
    with open_database() as database:
        rows = database.execute(query, values).fetchall()
    return {"runs": [{"id": row["id"], "created_at": row["created_at"], "tool": row["tool"],
        "operation": row["operation"], "inputs": json.loads(row["inputs_json"]),
        "result": json.loads(row["result_json"]), "provenance": row["provenance"],
        "project_id": row["project_id"],
        "project_snapshot": json.loads(row["project_snapshot_json"]) if row["project_snapshot_json"] else None} for row in rows]}


@app.post("/v1/integrations/{tool_id}/execute")
async def execute(tool_id: str, body: Operation, authorization: str | None = Header(default=None), session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    authorize(authorization, session_token)
    user = get_current_user(session_token) if session_token else None
    if tool_id == "pybamm":
        await pybamm_runtime()
    items = {tool["id"]: tool for tool in await inventory()}
    if tool_id not in items:
        raise HTTPException(status_code=404, detail="Unknown integration")
    if not items[tool_id]["available"]:
        state = items[tool_id]["status"]
        raise HTTPException(status_code=503, detail=f"{items[tool_id]['label']} is {state}", headers={"X-HELQIX-Tool-Status": state})
    project_id = None
    project_row = None
    project_context = None
    if body.inputs.get("project_id") is not None:
        project_row = owned_project(body.inputs["project_id"], user["id"] if user else None)
        project_id = project_row["id"]
        project_context = project_snapshot(project_row)
    if tool_id == "helqix_reality" and body.operation == "validate_reality":
        if project_row is None:
            raise HTTPException(status_code=422, detail="Reality validation must reference a saved project")
        saved_payload = json.loads(project_row["payload_json"])
        raw_reality = saved_payload.get("engineering_reality")
        if raw_reality is None:
            raise HTTPException(status_code=422, detail="This project has no saved domain-neutral engineering reality")
        try:
            validated = EngineeringReality.model_validate(raw_reality)
        except ValidationError as exc:
            issues = "; ".join(f"{'/'.join(str(part) for part in issue['loc'])}: {issue['msg']}" for issue in exc.errors())
            raise HTTPException(status_code=422, detail=f"Saved engineering reality is invalid: {issues}") from exc
        result_payload = {
            "status": "structurally_valid_draft",
            "schema": validated.schema_version,
            "domain": validated.domain,
            "revision": validated.revision,
            "counts": {
                "components": len(validated.components), "properties": sum(len(item.properties) for item in validated.components),
                "interfaces": len(validated.interfaces), "connections": len(validated.connections),
                "requirements": len(validated.requirements), "model_bindings": len(validated.model_bindings),
            },
            "engineering_assurance": "Structure and references only; no physical feasibility, solver compatibility, performance, or requirement satisfaction was assessed.",
        }
        run = persist_run(tool_id, body.operation, {"project_id": project_id}, result_payload,
                          "helqix-domain-contract", user["id"] if user else None, project_id, project_context)
        return {"run": run}
    if tool_id == "helqix_reality" and body.operation == "verify_bound_run":
        if project_row is None:
            raise HTTPException(status_code=422, detail="Verification must reference a saved project")
        unexpected = set(body.inputs) - {"project_id", "run_id"}
        if unexpected:
            raise HTTPException(status_code=422, detail=f"Unsupported verification inputs: {', '.join(sorted(unexpected))}")
        run_id = body.inputs.get("run_id")
        if not isinstance(run_id, str) or not run_id:
            raise HTTPException(status_code=422, detail="run_id is required")
        with open_database() as database:
            source_run = database.execute(
                "SELECT * FROM helqix_integration_runs WHERE id=? AND project_id=? AND user_id IS ?",
                (run_id, project_id, user["id"] if user else None),
            ).fetchone()
        if source_run is None:
            raise HTTPException(status_code=404, detail="Run not found in this project")
        if source_run["tool"] != "pybamm" or source_run["operation"] != "simulate_bound_cell":
            raise HTTPException(status_code=422, detail="Only completed project-bound PyBaMM cell runs can be evaluated by this verifier")
        source_result = json.loads(source_run["result_json"])
        if source_result.get("status") != "completed":
            raise HTTPException(status_code=422, detail="The source solver run did not complete")
        source_snapshot = json.loads(source_run["project_snapshot_json"]) if source_run["project_snapshot_json"] else None
        source_digest = (((source_snapshot or {}).get("models") or {}).get("engineering_reality") or {}).get("sha256")
        current_digest = (((project_context or {}).get("models") or {}).get("engineering_reality") or {}).get("sha256")
        same_model_revision = bool(source_digest and source_digest == current_digest)
        try:
            reality = EngineeringReality.model_validate(json.loads(project_row["payload_json"]).get("engineering_reality"))
        except ValidationError as exc:
            raise HTTPException(status_code=422, detail="Saved engineering reality is invalid") from exc
        requirements = [item for item in reality.requirements if item.review_status == "accepted" and item.verification_rule]
        if not requirements:
            raise HTTPException(status_code=422, detail="Accept at least one requirement with an explicit supported verification rule")
        checks = []
        operators = {
            ">=": lambda observed, threshold: observed >= threshold,
            "<=": lambda observed, threshold: observed <= threshold,
            ">": lambda observed, threshold: observed > threshold,
            "<": lambda observed, threshold: observed < threshold,
            "==": lambda observed, threshold: math.isclose(observed, threshold, rel_tol=1e-9, abs_tol=1e-12),
        }
        for requirement in requirements:
            rule = requirement.verification_rule
            observed = source_result.get(rule.metric) if same_model_revision else None
            verdict = "inconclusive"
            if isinstance(observed, (int, float)) and not isinstance(observed, bool) and math.isfinite(float(observed)):
                verdict = "pass" if operators[rule.operator](float(observed), float(rule.threshold)) else "fail"
            condition = f"{rule.metric} {rule.operator} {rule.threshold} {rule.unit}"
            check_payload = {
                "schema": "helqix.verification-check.v1",
                "requirement_id": requirement.id,
                "code": requirement.id,
                "requirementTitle": requirement.statement,
                "governingStandard": requirement.source_ref or "User-defined accepted criterion",
                "conditionFormula": condition,
                "metric": rule.metric, "operator": rule.operator, "threshold": rule.threshold, "unit": rule.unit,
                "observed_value": float(observed) if isinstance(observed, (int, float)) and not isinstance(observed, bool) and math.isfinite(float(observed)) else None,
                "observedValue": f"{observed} {rule.unit}" if observed is not None else "Not evaluated: source model revision mismatch or metric unavailable",
                "limitThreshold": f"{rule.operator} {rule.threshold} {rule.unit}",
                "marginPercent": ((float(observed) - float(rule.threshold)) / abs(float(rule.threshold)) * 100)
                    if verdict != "inconclusive" and float(rule.threshold) != 0 else None,
                "verdict": verdict,
                "method": "explicit saved threshold compared with a recorded PyBaMM SPM / Chen2020 model output",
                "source_run_id": run_id,
                "source_model_sha256": source_digest,
                "current_model_sha256": current_digest,
            }
            try:
                ExplicitVerificationCheck.model_validate(check_payload)
            except ValidationError as exc:
                raise HTTPException(status_code=502, detail="Verification service produced an invalid check record") from exc
            checks.append(persist_project_resource("verifications", check_payload,
                          user["id"] if user else None, project_id))
        verdicts = [item["payload"]["verdict"] for item in checks]
        result_payload = {
            "status": "completed" if any(value in {"pass", "fail"} for value in verdicts) else "inconclusive",
            "source_run_id": run_id, "source_model_revision_matches": same_model_revision,
            "checks": checks,
            "assurance": "Evaluates only the explicit accepted thresholds included here. Passing these checks does not certify the product or validate other requirements.",
        }
        return {"run": persist_run(tool_id, body.operation,
                {"project_id": project_id, "run_id": run_id}, result_payload,
                "helqix-explicit-threshold-check", user["id"] if user else None, project_id, project_context)}
    if tool_id == "blender" and body.operation == "create_quadrotor":
        if project_id is None:
            raise HTTPException(status_code=422, detail="Blender generation must reference a saved project design")
        project = owned_project(project_id, user["id"] if user else None)
        project_context = project_snapshot(project)
        project_payload = json.loads(project["payload_json"])
        if not isinstance(project_payload, dict) or "specification" not in project_payload:
            raise HTTPException(status_code=422, detail="Save a valid quadrotor design in this project before generating an asset")
        try:
            specification = DroneSpecification.model_validate(project_payload["specification"])
        except ValidationError as exc:
            issues = "; ".join(f"{'/'.join(str(part) for part in issue['loc'])}: {issue['msg']}" for issue in exc.errors())
            raise HTTPException(status_code=422, detail=f"Saved drone design is invalid: {issues}") from exc
        authoritative_inputs = {**body.inputs, "project_id": project_id,
                                "specification": specification.model_dump(by_alias=True)}
        output = workspace_output(body.inputs.get("output_path"), suffix=".blend")
        spec_json = json.dumps(specification.model_dump(by_alias=True), separators=(",", ":"), allow_nan=False)
        script = r'''import bpy, json, math, os, sys
from mathutils import Vector
args=sys.argv[sys.argv.index('--')+1:]
spec=json.loads(args[0]); output=os.path.abspath(args[1])
scene=bpy.context.scene
scene.unit_settings.system='METRIC'; scene.unit_settings.scale_length=1.0
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
collection=bpy.data.collections.new('HELQIX Drone Design Draft'); scene.collection.children.link(collection)
created=[]
def material(name, color, metallic=0.25, roughness=0.35):
 m=bpy.data.materials.new(name); m.diffuse_color=(*color,1.0); m.use_nodes=True
 bsdf=m.node_tree.nodes.get('Principled BSDF')
 if bsdf: bsdf.inputs['Base Color'].default_value=(*color,1.0); bsdf.inputs['Metallic'].default_value=metallic; bsdf.inputs['Roughness'].default_value=roughness
 return m
carbon=material('Draft | carbon composite appearance',(0.045,0.095,0.13),0.55,0.34)
accent=material('Draft | HELQIX cyan appearance',(0.02,0.58,0.64),0.45,0.3)
motor_mat=material('Draft | motor housing appearance',(0.12,0.16,0.19),0.6,0.28)
battery_mat=material('Draft | battery enclosure appearance',(0.66,0.19,0.11),0.3,0.5)
prop_mat=material('Draft | propeller appearance',(0.12,0.42,0.52),0.25,0.38)
def register(obj, label):
 for c in list(obj.users_collection): c.objects.unlink(obj)
 collection.objects.link(obj); obj['helqix_role']=label; obj['helqix_status']='parametric_visualization_draft'; created.append(obj); return obj
def cube(name, dims, loc, mat, role):
 bpy.ops.mesh.primitive_cube_add(size=1, location=loc); obj=bpy.context.object; obj.name=name; obj.dimensions=dims; bpy.ops.object.transform_apply(location=False, rotation=False, scale=True); obj.data.materials.append(mat); obj=register(obj,role); obj.modifiers.new('Edge highlights (visual only)','BEVEL').width=0.025; obj.modifiers[-1].segments=2; return obj
def cylinder(name, radius, depth, loc, mat, role, direction=None):
 bpy.ops.mesh.primitive_cylinder_add(vertices=24, radius=radius, depth=depth, location=loc); obj=bpy.context.object; obj.name=name
 if direction is not None: obj.rotation_mode='QUATERNION'; obj.rotation_quaternion=Vector((0,0,1)).rotation_difference(direction.normalized())
 obj.data.materials.append(mat); return register(obj,role)
def rod(name, start, end, radius, mat, role):
 a=Vector(start); b=Vector(end); delta=b-a; return cylinder(name,radius,delta.length,(a+b)*0.5,mat,role,delta)
frame=float(spec['frameDiagonalMm'])/1000.0
offset=frame/(2.0*math.sqrt(2.0))
prop_diameter=float(spec['propellerDiameterIn'])*0.0254
cube('Center frame / avionics bay',(0.19,0.15,0.075),(0,0,0.03),carbon,'airframe')
for index,(sx,sy) in enumerate(((1,1),(-1,1),(-1,-1),(1,-1))):
 x=sx*offset; y=sy*offset; loc=Vector((x,y,0.02)); center=Vector((0,0,0.02)); name=f'Rotor {index+1}'
 rod(f'{name} | frame arm',center,loc,0.018,accent if index in (0,2) else carbon,'airframe_arm')
 cylinder(f'{name} | motor',0.032,0.05,(x,y,0.065),motor_mat,'motor_housing')
 hub=cylinder(f'{name} | propeller hub',0.012,0.012,(x,y,0.096),accent,'propeller_hub')
 blade_length=prop_diameter*0.43
 for blade in range(2):
  theta=blade*math.pi/2
  obj=cube(f'{name} | blade {blade+1}',(blade_length,0.018,0.003),(x,y,0.105),prop_mat,'propeller_visual')
  obj.rotation_euler[2]=theta
batt_w=0.17; batt_d=0.12; batt_h=0.045
battery=cube('Battery enclosure | visual proxy',(batt_w,batt_d,batt_h),(0,0,-0.07),battery_mat,'battery_enclosure')
battery['configured_mass_kg']=float(spec['batteryMassKg']); battery['capacity_mAh']=float(spec['batteryCapacityMah']); battery['nominal_voltage_V']=float(spec['batteryVoltageV'])
for index,side in enumerate((-1,1)):
 rod(f'Landing strut {index+1}',(side*0.055,0,-0.02),(side*0.085,0,-0.15),0.008,carbon,'landing_gear_visual')
 rod(f'Landing foot {index+1}',(side*0.11,-0.09,-0.15),(side*0.11,0.09,-0.15),0.009,carbon,'landing_gear_visual')
for obj in created:
 obj['source_schema']='helqix.drone-specification.v1'
 obj['geometry_dimensions_basis']='User-entered frame diagonal and propeller diameter; other dimensions are visual placeholders.'
obj=bpy.data.objects.new('HELQIX design draft metadata',None); collection.objects.link(obj); obj.empty_display_type='CUBE'; obj.empty_display_size=0.03
obj['specification_json']=json.dumps(spec,sort_keys=True); obj['total_configured_mass_kg']=float(spec['dryMassKg'])+float(spec['payloadMassKg'])+float(spec['batteryMassKg'])
obj['engineering_status']='visual draft only; no CAD validation or flight-dynamics simulation'
bpy.ops.mesh.primitive_plane_add(size=max(1.5,frame*2.5), location=(0,0,-0.19)); ground=bpy.context.object; ground.name='Presentation ground (not part of vehicle)'; ground.data.materials.append(material('Presentation | ground',(0.025,0.035,0.05),0,0.8)); register(ground,'presentation_only')
bpy.ops.object.camera_add(location=(1.9,-2.2,1.65)); camera=bpy.context.object; camera.name='Presentation camera'; target=Vector((0,0,0)); camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler(); camera.data.lens=52; scene.camera=camera
bpy.ops.object.light_add(type='AREA', location=(1.2,-1.0,2.2)); key=bpy.context.object; key.name='Presentation key light'; key.data.energy=420; key.data.shape='DISK'; key.data.size=2.0
bpy.ops.object.light_add(type='AREA', location=(-1.5,1.0,1.2)); fill=bpy.context.object; fill.name='Presentation fill light'; fill.data.energy=180; fill.data.color=(0.25,0.72,1.0); fill.data.size=1.7
scene.render.resolution_x=1280; scene.render.resolution_y=900; scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'; scene.render.filepath=os.path.splitext(output)[0]+'.png'
scene.render.engine='BLENDER_EEVEE'
bpy.ops.wm.save_as_mainfile(filepath=output,check_existing=False)
mesh_objects=[o for o in collection.objects if o.type=='MESH']
print('HELQIX_RESULT='+json.dumps({'asset_path':output,'tool_version':bpy.app.version_string,'design':'quadrotor','schema':spec['schema'],'mesh_count':len(mesh_objects),'object_count':len(created),'configured_mass_kg':obj['total_configured_mass_kg'],'dimensions_basis':{'frame_diagonal_m':frame,'propeller_diameter_m':prop_diameter},'file_bytes':os.path.getsize(output),'provenance':'Blender background scene generated from saved project inputs','engineering_status':'visual draft only; not CAD validated, flight simulated, or airworthiness assessed'}))
'''
        with tempfile.NamedTemporaryFile("w", suffix=".py", encoding="utf-8", delete=False) as f:
            f.write(script); script_path = Path(f.name)
        try:
            result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=authoritative_inputs,
                provenance="blender-cli", user_id=user["id"] if user else None, project_id=project_id,
                project_snapshot=project_context,
                args=[blender_program() or BLENDER_CMD, "--background", "--factory-startup", "--python", str(script_path), "--", spec_json, str(output)],
                timeout=180, failure_message="Blender could not generate the drone design draft")
        finally:
            script_path.unlink(missing_ok=True)
        result_payload = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=authoritative_inputs, provenance="blender-cli", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            failure_message="Blender did not return a design report")
        if not output.is_file() or output.stat().st_size == 0:
            failed = persist_run(tool_id, body.operation, authoritative_inputs,
                {"status": "failed", "failure_stage": "output_validation", "error": "Blender finished without producing a non-empty design file.",
                 "stdout": result["stdout"], "stderr": result["stderr"]}, "blender-cli",
                user["id"] if user else None, project_id, project_context)
            raise HTTPException(status_code=502, detail={"message": "Blender finished without producing a valid design file", "run_id": failed["id"]})
        result_payload["project_id"] = project_id
        run = persist_run(tool_id, body.operation, authoritative_inputs, result_payload, "blender-cli",
                          user["id"] if user else None, project_id, project_context)
        return {"run": run}

    if tool_id == "blender" and body.operation == "inspect_asset":
        asset = workspace_file(body.inputs.get("asset_path"), suffixes={".blend", ".obj", ".stl", ".fbx", ".gltf", ".glb", ".ply"})
        script = r'''import bpy, json, sys
p=sys.argv[-1]
if p.lower().endswith('.blend'): bpy.ops.wm.open_mainfile(filepath=p)
elif p.lower().endswith('.obj'): bpy.ops.wm.obj_import(filepath=p)
elif p.lower().endswith('.stl'): bpy.ops.wm.stl_import(filepath=p)
elif p.lower().endswith('.fbx'): bpy.ops.import_scene.fbx(filepath=p)
elif p.lower().endswith(('.gltf','.glb')): bpy.ops.import_scene.gltf(filepath=p)
elif p.lower().endswith('.ply'): bpy.ops.wm.ply_import(filepath=p)
unit_system=bpy.context.scene.unit_settings.system
unit_scale=float(bpy.context.scene.unit_settings.scale_length) if unit_system != 'NONE' else None
objects=[]
for o in bpy.context.scene.objects:
 if o.type=='MESH':
  dims=[round(float(v),6) for v in o.dimensions]
  objects.append({'name':o.name,'vertices':len(o.data.vertices),'faces':len(o.data.polygons),'dimensions_blender_units':dims})
print('HELQIX_RESULT='+json.dumps({'asset':p,'tool_version':bpy.app.version_string,'mesh_count':len(objects),'unit_system':unit_system,'meters_per_blender_unit':unit_scale,'objects':objects}))
'''
        with tempfile.NamedTemporaryFile("w", suffix=".py", encoding="utf-8", delete=False) as f:
            f.write(script); script_path = Path(f.name)
        try:
            result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=body.inputs,
                provenance="blender-cli", user_id=user["id"] if user else None, project_id=project_id,
                project_snapshot=project_context,
                args=[blender_program() or BLENDER_CMD, "--background", "--python", str(script_path), "--", str(asset)],
                timeout=180, failure_message="Blender could not inspect this asset")
        finally:
            script_path.unlink(missing_ok=True)
        result_payload = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=body.inputs, provenance="blender-cli", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            failure_message="Blender completed without returning an asset report")
        run = persist_run(tool_id, body.operation, body.inputs, result_payload, "blender-cli", user["id"] if user else None, project_id, project_context)
        return {"run": run}

    if tool_id == "pybamm" and body.operation == "simulate_discharge":
        current = body.inputs.get("current_a", 5.0)
        cutoff = body.inputs.get("cutoff_voltage_v", 3.0)
        if isinstance(current, bool) or not isinstance(current, (int, float)) or not 0.05 <= current <= 100:
            raise HTTPException(status_code=422, detail="current_a must be between 0.05 and 100 A")
        if isinstance(cutoff, bool) or not isinstance(cutoff, (int, float)) or not 2.5 <= cutoff <= 4.0:
            raise HTTPException(status_code=422, detail="cutoff_voltage_v must be between 2.5 and 4.0 V")
        script = """import json, pybamm\nm=pybamm.lithium_ion.SPM()\np=pybamm.ParameterValues('Chen2020')\ne=pybamm.Experiment([f'Discharge at {float(__CURRENT__)} A until {float(__CUTOFF__)} V'])\ns=pybamm.Simulation(m, parameter_values=p, experiment=e).solve()\nt=s['Time [h]'].entries; v=s['Voltage [V]'].entries; q=s['Discharge capacity [A.h]'].entries\nprint('HELQIX_RESULT='+json.dumps({'model':'SPM','parameter_set':'Chen2020','tool_version':pybamm.__version__,'current_a':float(__CURRENT__),'cutoff_voltage_v':float(__CUTOFF__),'duration_h':float(t[-1]),'discharged_capacity_ah':float(q[-1]),'final_voltage_v':float(v[-1]),'sample_count':len(t)}))\n""".replace("__CURRENT__", repr(float(current))).replace("__CUTOFF__", repr(float(cutoff)))
        result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=body.inputs,
            provenance="pybamm", user_id=user["id"] if user else None, project_id=project_id,
            project_snapshot=project_context, args=[PYTHON_RUNTIME, "-c", script], timeout=600,
            failure_message="PyBaMM simulation failed")
        result_payload = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=body.inputs, provenance="pybamm", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            failure_message="PyBaMM completed without a simulation report")
        return {"run": persist_run(tool_id, body.operation, body.inputs, result_payload, "pybamm", user["id"] if user else None, project_id, project_context)}

    if tool_id == "pybamm" and body.operation == "simulate_ev_drive_cycle":
        if project_row is None:
            raise HTTPException(status_code=422, detail="EV simulation requires a saved project")
        if set(body.inputs) != {"project_id"}:
            raise HTTPException(status_code=422, detail="EV simulation inputs come from the saved project; only project_id is accepted")
        saved_payload = json.loads(project_row["payload_json"])
        raw_config = saved_payload.get("ev_vehicle")
        if not isinstance(raw_config, dict):
            raise HTTPException(status_code=422, detail="Save an EV vehicle and battery-pack configuration to this project first")
        try:
            config = EVBatteryVehicleConfiguration.model_validate(raw_config)
        except ValidationError as exc:
            issues = "; ".join(f"{'/'.join(str(part) for part in issue['loc'])}: {issue['msg']}" for issue in exc.errors())
            raise HTTPException(status_code=422, detail=f"Saved EV configuration is invalid: {issues}") from exc
        from ev_vehicle_solver import build_drive_cycle_load
        try:
            load = build_drive_cycle_load(config.model_dump())
        except ValueError as exc:
            raise HTTPException(status_code=422, detail=str(exc)) from exc
        # Keep the exact validated design inputs with the immutable execution
        # record. The project snapshot stores a digest, not historical payloads;
        # a hash alone is insufficient to reproduce or compare this run later.
        run_inputs = {
            "project_id": project_id,
            "ev_vehicle_configuration": config.model_dump(by_alias=True, mode="json"),
        }
        cycle_json = json.dumps({"config": config.model_dump(), "load": load}, separators=(",", ":"), allow_nan=False)
        cycle_b64 = __import__("base64").b64encode(cycle_json.encode("utf-8")).decode("ascii")
        script = "\n".join([
            "import base64,json,math,numpy as np,pybamm",
            'd=json.loads(base64.b64decode("__DATA__")); c=d["config"]; load=d["load"]; pack=c["battery_pack"]',
            't=np.array([x["time_s"] for x in load]); i=np.array([x["cell_current_a"] for x in load])',
            'p=pybamm.ParameterValues(c["parameter_set"]); base_capacity=float(p["Nominal cell capacity [A.h]"]); p.update({"Electrode width [m]":float(p["Electrode width [m]"])*float(pack["cell_capacity_ah"])/base_capacity,"Nominal cell capacity [A.h]":float(pack["cell_capacity_ah"])})',
            'p.set_initial_state(float(pack["initial_soc"]))',
            'm=pybamm.lithium_ion.SPM(); p.update({"Current function [A]":pybamm.Interpolant(t,i,pybamm.t)})',
            'm.events.append(pybamm.Event("Configured cell cutoff",m.variables["Voltage [V]"]-float(pack["cell_cutoff_voltage_v"])))',
            's=pybamm.Simulation(m,parameter_values=p).solve(t_eval=t); actual_t=s["Time [s]"].entries',
            'v=s["Voltage [V]"].entries; q=s["Discharge capacity [A.h]"].entries',
            'stride=max(1,math.ceil(len(actual_t)/40)); ix=list(range(0,len(actual_t),stride)); ix.append(len(actual_t)-1) if ix[-1]!=len(actual_t)-1 else None',
            'speed=np.interp(actual_t,t,[x["speed_m_s"] for x in load]); distance=np.interp(actual_t,t,[x["distance_m"] for x in load]); power=np.interp(actual_t,t,[x["pack_power_w"] for x in load]); amps=np.interp(actual_t,t,i)',
            'n=pack["series_cells"]; z=pack["parallel_cells"]',
            'series=[{"time_s":float(actual_t[k]),"speed_m_s":float(speed[k]),"distance_m":float(distance[k]),"cell_voltage_v":float(v[k]),"pack_voltage_v":float(v[k]*n),"pack_current_a":float(amps[k]*z),"cell_current_a":float(amps[k]),"pack_power_w":float(power[k]),"cell_capacity_ah":float(q[k])} for k in ix]',
            'out={"status":"completed","model":"pybamm.lithium-ion.spm","pybamm_version":pybamm.__version__,"parameter_set":c["parameter_set"],"modeled_cell_capacity_ah":float(pack["cell_capacity_ah"]),"vehicle_name":c["vehicle_name"],"series_cells":n,"parallel_cells":z,"drive_cycle_duration_s":float(actual_t[-1]),"requested_drive_cycle_duration_s":float(t[-1]),"distance_m":float(distance[-1]),"peak_pack_power_w":max(x["pack_power_w"] for x in load),"peak_cell_current_a":float(max(i)),"minimum_cell_voltage_v":float(min(v)),"final_cell_voltage_v":float(v[-1]),"final_pack_voltage_v":float(v[-1]*n),"stopped_at_cell_cutoff":bool(actual_t[-1]<t[-1]-1e-6),"sample_count":len(actual_t),"series":series,"applicability":"Prescribed-speed longitudinal load coupled to a single-cell SPM and scaled by identical series/parallel counts. Requested capacity scales Chen2020 electrode width; chemistry and cell characteristics remain Chen2020 assumptions, not actual-cell characterization. Pack thermal gradients, cell mismatch, BMS/contactors, regeneration, tire dynamics, and free-running acceleration are not modeled."}',
            'print("HELQIX_RESULT="+json.dumps(out,allow_nan=False))'
        ]).replace("__DATA__", cycle_b64)
        result = await run_tool_process(tool=tool_id, operation=body.operation,
            inputs=run_inputs, provenance="pybamm-spm-chen2020-ev-drive-cycle-v1",
            user_id=user["id"] if user else None, project_id=project_id,
            project_snapshot=project_context, args=[PYTHON_RUNTIME, "-c", script], timeout=600,
            failure_message="EV drive-cycle PyBaMM run failed")
        result_payload = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=run_inputs, provenance="pybamm-spm-chen2020-ev-drive-cycle-v1",
            user_id=user["id"] if user else None, project_id=project_id,
            project_snapshot=project_context,
            failure_message="PyBaMM completed without an EV simulation report")
        result_payload["status"] = "completed"
        result_payload["model_input_sha256"] = project_context["models"]["ev_vehicle"]["sha256"]
        return {"run": persist_run(tool_id, body.operation, run_inputs, result_payload,
                                    "pybamm-spm-chen2020-ev-drive-cycle-v1", user["id"] if user else None,
                                    project_id, project_context)}

    if tool_id == "pybamm" and body.operation == "simulate_bound_cell":
        if project_row is None:
            raise HTTPException(status_code=422, detail="A model-bound run must reference a saved project")
        unexpected = set(body.inputs) - {"project_id", "binding_id", "branch_id"}
        if unexpected:
            raise HTTPException(status_code=422, detail=f"Model-bound inputs must be saved on the component; unsupported overrides: {', '.join(sorted(unexpected))}")
        saved_payload = json.loads(project_row["payload_json"])
        raw_reality = saved_payload.get("engineering_reality")
        if raw_reality is None:
            raise HTTPException(status_code=422, detail="This project has no saved engineering reality")
        try:
            reality = EngineeringReality.model_validate(raw_reality)
        except ValidationError as exc:
            raise HTTPException(status_code=422, detail="Saved engineering reality is invalid") from exc
        branch = None
        branch_changes: list[dict[str, Any]] = []
        branch_id = body.inputs.get("branch_id")
        if branch_id is not None:
            if not isinstance(branch_id, str) or not branch_id:
                raise HTTPException(status_code=422, detail="branch_id must be a saved branch identifier")
            with open_database() as database:
                branch_row = database.execute(
                    "SELECT * FROM helqix_resources WHERE id=? AND resource_type='branches' AND project_id=? AND user_id IS ?",
                    (branch_id, project_id, user["id"] if user else None),
                ).fetchone()
            if branch_row is None:
                raise HTTPException(status_code=404, detail="Engineering branch not found in this project")
            try:
                branch = EngineeringBranch.model_validate(json.loads(branch_row["payload_json"]))
            except ValidationError as exc:
                raise HTTPException(status_code=422, detail="Saved engineering branch is invalid") from exc
            if branch.base_reality_revision != reality.revision:
                raise HTTPException(status_code=409, detail="This branch is based on an older engineering-reality revision; recreate it from the current model")
        bindings = [binding for binding in reality.model_bindings
                    if binding.model_id == "pybamm.lithium-ion.spm" and binding.model_version == "Chen2020"
                    and (binding.scope_component_id is not None)
                    and (body.inputs.get("binding_id") is None or binding.id == body.inputs["binding_id"])]
        if len(bindings) != 1:
            raise HTTPException(status_code=422, detail="Select exactly one scoped model binding for pybamm.lithium-ion.spm version Chen2020")
        binding = bindings[0]
        component = next((item for item in reality.components if item.id == binding.scope_component_id), None)
        if component is None or component.component_type.lower() not in {"battery_cell", "lithium_ion_cell"}:
            raise HTTPException(status_code=422, detail="The bound component must be typed battery_cell or lithium_ion_cell; this adapter does not simulate packs")
        properties = {item.name: item for item in component.properties}
        if branch is not None:
            if any(change.component_id != component.id for change in branch.changes):
                raise HTTPException(status_code=422, detail="This adapter accepts branch changes only for the bound battery cell")
            for change in branch.changes:
                if change.property_name not in {"current_a", "cutoff_voltage_v"}:
                    raise HTTPException(status_code=422, detail=f"PyBaMM SPM adapter cannot execute branch property '{change.property_name}'")
                previous = properties.get(change.property_name)
                if previous is None or not isinstance(change.value, (int, float)) or isinstance(change.value, bool):
                    raise HTTPException(status_code=422, detail=f"Branch property '{change.property_name}' must replace a saved numeric cell input")
                if change.unit != previous.unit:
                    raise HTTPException(status_code=422, detail=f"Branch property '{change.property_name}' unit must match the saved value ({previous.unit})")
                properties[change.property_name] = previous.model_copy(update={"value": change.value})
                branch_changes.append({"component_id": change.component_id, "property_name": change.property_name,
                                       "base_value": previous.value, "value": change.value, "unit": change.unit})
        current_property = properties.get("current_a")
        cutoff_property = properties.get("cutoff_voltage_v")
        if current_property is None or cutoff_property is None:
            raise HTTPException(status_code=422, detail="The bound cell must define current_a and cutoff_voltage_v properties")
        current = current_property.value
        cutoff = cutoff_property.value
        if (isinstance(current, bool) or not isinstance(current, (int, float)) or not 0.05 <= current <= 100
                or current_property.unit != "A"):
            raise HTTPException(status_code=422, detail="current_a must be a numeric value from 0.05 to 100 with unit A")
        if (isinstance(cutoff, bool) or not isinstance(cutoff, (int, float)) or not 2.5 <= cutoff <= 4.0
                or cutoff_property.unit != "V"):
            raise HTTPException(status_code=422, detail="cutoff_voltage_v must be a numeric value from 2.5 to 4.0 with unit V")
        script = """import json, math, pybamm\nm=pybamm.lithium_ion.SPM()\np=pybamm.ParameterValues('Chen2020')\ne=pybamm.Experiment([f'Discharge at {float(__CURRENT__)} A until {float(__CUTOFF__)} V'])\ns=pybamm.Simulation(m, parameter_values=p, experiment=e).solve()\nt=s['Time [h]'].entries; v=s['Voltage [V]'].entries; q=s['Discharge capacity [A.h]'].entries\nstride=max(1,math.ceil(len(t)/201)); indices=list(range(0,len(t),stride))\nif indices[-1] != len(t)-1: indices.append(len(t)-1)\nseries=[{'time_h':float(t[i]),'voltage_v':float(v[i]),'capacity_ah':float(q[i])} for i in indices]\nprint('HELQIX_RESULT='+json.dumps({'tool_version':pybamm.__version__,'duration_h':float(t[-1]),'discharged_capacity_ah':float(q[-1]),'final_voltage_v':float(v[-1]),'sample_count':len(t),'series':series}))\n""".replace("__CURRENT__", repr(float(current))).replace("__CUTOFF__", repr(float(cutoff)))
        run_inputs = {"project_id": project_id, "binding_id": binding.id,
                      **({"branch_id": branch_id} if branch is not None else {})}
        result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=run_inputs,
            provenance="pybamm-spm-chen2020-bound-engineering-reality",
            user_id=user["id"] if user else None, project_id=project_id,
            project_snapshot=project_context, args=[PYTHON_RUNTIME, "-c", script], timeout=600,
            failure_message="Bound PyBaMM single-cell run failed")
        raw_result = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=run_inputs, provenance="pybamm-spm-chen2020-bound-engineering-reality",
            user_id=user["id"] if user else None, project_id=project_id,
            project_snapshot=project_context,
            failure_message="PyBaMM completed without a model-bound result report")
        result_payload = {
            "status": "completed", "model": binding.model_id, "model_version": binding.model_version,
            "declared_fidelity": binding.declared_fidelity, "domain": reality.domain,
            "component_id": component.id, "component_name": component.name,
            "inputs": {"current_a": current, "current_origin": current_property.origin,
                       "cutoff_voltage_v": cutoff, "cutoff_voltage_origin": cutoff_property.origin},
            "branch": ({"id": branch_id, "name": branch.name, "base_reality_revision": branch.base_reality_revision,
                        "changes": branch_changes,
                        "sha256": hashlib.sha256(json.dumps(branch.model_dump(by_alias=True), sort_keys=True,
                                                          separators=(",", ":"), allow_nan=False).encode("utf-8")).hexdigest()}
                       if branch is not None else None),
            **raw_result,
            "applicability": "Single-cell discharge SPM with Chen2020 parameters; use only when the physical cell matches that parameterization. This is not a battery-pack, system-load, thermal, or product-level prediction.",
        }
        run = persist_run(tool_id, body.operation, run_inputs, result_payload,
                          "pybamm-spm-chen2020-bound-engineering-reality", user["id"] if user else None,
                          project_id, project_context)
        return {"run": run}

    if tool_id == "freecad" and body.operation == "inspect_step":
        step_file = workspace_file(body.inputs.get("asset_path"), suffixes={".step", ".stp"})
        macro = r'''import FreeCAD as App, Part, json
doc=App.newDocument('HELQIXInspect')
shape=Part.read(r''' + repr(str(step_file)) + r''')
print('HELQIX_RESULT='+json.dumps({'tool_version':App.Version(),'asset':r''' + repr(str(step_file)) + r''','volume_mm3':shape.Volume,'surface_area_mm2':shape.Area,'solid_count':len(shape.Solids),'valid':shape.isValid()}))
App.closeDocument(doc.Name)
'''
        with tempfile.NamedTemporaryFile("w", suffix=".FCMacro", encoding="utf-8", delete=False) as f:
            f.write(macro); macro_path = Path(f.name)
        try:
            cmd = freecad_program() or FREECAD_CMD
            result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=body.inputs,
                provenance="freecad-step", user_id=user["id"] if user else None, project_id=project_id,
                project_snapshot=project_context, args=[cmd, str(macro_path)], timeout=180,
                failure_message="FreeCAD could not inspect this STEP model")
        finally:
            macro_path.unlink(missing_ok=True)
        result_payload = decode_tool_report(result, tool=tool_id, operation=body.operation,
            inputs=body.inputs, provenance="freecad-step", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            failure_message="FreeCAD completed without a geometry report")
        return {"run": persist_run(tool_id, body.operation, body.inputs, result_payload, "freecad-step", user["id"] if user else None, project_id, project_context)}

    if tool_id == "openfoam" and body.operation == "run_case":
        raw = body.inputs.get("case_dir")
        if not isinstance(raw, str): raise HTTPException(status_code=422, detail="case_dir is required")
        case = (ROOT / raw).resolve()
        if not case.is_relative_to(ROOT) or not (case / "system" / "controlDict").is_file():
            raise HTTPException(status_code=422, detail="OpenFOAM case must be inside the workspace and contain system/controlDict")
        solver = body.inputs.get("solver", "simpleFoam")
        if solver not in FOAM_SOLVERS: raise HTTPException(status_code=422, detail="Unsupported OpenFOAM solver")
        timeout = body.inputs.get("timeout_s", 1800)
        if isinstance(timeout, bool) or not isinstance(timeout, int) or not 10 <= timeout <= 3600:
            raise HTTPException(status_code=422, detail="timeout_s must be an integer from 10 to 3600")
        mesh = await run_tool_process(tool=tool_id, operation=body.operation, inputs=body.inputs,
            provenance="openfoam-blockmesh-cli", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            args=[executable("blockMesh") or "blockMesh", "-case", str(case)], cwd=case,
            timeout=timeout, failure_message="OpenFOAM blockMesh failed")
        solver_cmd = executable(solver)
        if not solver_cmd: raise HTTPException(status_code=503, detail=f"OpenFOAM solver {solver} is unavailable")
        result = await run_tool_process(tool=tool_id, operation=body.operation, inputs=body.inputs,
            provenance="openfoam-cli", user_id=user["id"] if user else None,
            project_id=project_id, project_snapshot=project_context,
            args=[solver_cmd, "-case", str(case)], cwd=case,
            timeout=timeout, failure_message=f"OpenFOAM {solver} failed")
        result_payload = {"case_dir": str(case), "solver": solver, "mesh": mesh, "solution": result}
        return {"run": persist_run(tool_id, body.operation, body.inputs, result_payload, "openfoam-cli", user["id"] if user else None, project_id, project_context)}

    if tool_id == "px4_gazebo" and body.operation == "start_sitl":
        model = body.inputs.get("model", "gz_x500")
        if model not in PX4_MODELS: raise HTTPException(status_code=422, detail="Unsupported PX4 Gazebo model")
        try:
            process = await asyncio.create_subprocess_exec("make", "px4_sitl", model, cwd=str(PX4_DIR),
                stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL,
                creationflags=getattr(asyncio.subprocess, "CREATE_NEW_PROCESS_GROUP", 0))
        except OSError:
            failed = persist_run(tool_id, body.operation, body.inputs,
                {"status": "failed", "failure_stage": "process_start", "error": "PX4 SITL process could not be started."},
                "px4-sitl+gazebo", user["id"] if user else None, project_id, project_context)
            raise HTTPException(status_code=503, detail={"message": "PX4 SITL process could not be started", "run_id": failed["id"]}) from None
        session = SitlSession(process, model, user["id"] if user else None)
        async with sessions_lock: sessions[session.id] = session
        session_data = session.as_dict()
        return {"run": persist_run(tool_id, body.operation, body.inputs,
            {"status": session_data["status"], "session": session_data}, "px4-sitl+gazebo",
            user["id"] if user else None, project_id, project_context)}

    raise HTTPException(status_code=422, detail="Operation is not supported by this integration")


@app.get("/v1/integrations/px4_gazebo/sessions")
async def list_sessions(authorization: str | None = Header(default=None), session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    authorize(authorization, session_token)
    user = get_current_user(session_token) if session_token else None
    user_id = user["id"] if user else None
    async with sessions_lock:
        return {"sessions": [session.as_dict() for session in sessions.values() if session.user_id == user_id]}


@app.delete("/v1/integrations/px4_gazebo/sessions/{session_id}")
async def stop_session(session_id: str, authorization: str | None = Header(default=None), session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    authorize(authorization, session_token)
    user = get_current_user(session_token) if session_token else None
    async with sessions_lock: session = sessions.get(session_id)
    if not session: raise HTTPException(status_code=404, detail="Unknown PX4 session")
    if session.user_id != (user["id"] if user else None):
        raise HTTPException(status_code=404, detail="Unknown PX4 session")
    if session.process.returncode is None:
        session.process.terminate()
        try: await asyncio.wait_for(session.process.wait(), timeout=8)
        except asyncio.TimeoutError:
            session.process.kill(); await session.process.wait()
    return {"session": session.as_dict()}


def set_session_cookie(response: Response, token: str) -> None:
    response.set_cookie(COOKIE_NAME, token, max_age=14 * 24 * 60 * 60, httponly=True,
        secure=COOKIE_SECURE, samesite="lax", path="/")


def create_session(database: sqlite3.Connection, user_id: str, response: Response) -> None:
    token = secrets.token_urlsafe(36)
    now = time.time()
    database.execute("INSERT INTO helqix_sessions(token_hash,user_id,expires_at,created_at) VALUES(?,?,?,?)",
        (hashlib.sha256(token.encode()).hexdigest(), user_id, now + 14 * 24 * 60 * 60, now))
    set_session_cookie(response, token)


@app.post("/v1/auth/signup", status_code=201)
async def sign_up(body: SignUpRequest, response: Response) -> dict[str, Any]:
    name, email, organization = body.name.strip(), body.email.strip().lower(), body.organization.strip()
    if not name or not re.fullmatch(r"[^@\s]+@[^@\s]+\.[^@\s]+", email):
        raise HTTPException(status_code=422, detail="Enter a valid name and email address")
    salt = secrets.token_bytes(16)
    password_hash = await asyncio.to_thread(hashlib.pbkdf2_hmac, "sha256", body.password.encode(), salt, PASSWORD_ITERATIONS)
    user_id, now = str(uuid.uuid4()), time.time()
    try:
        with open_database() as database:
            database.execute("INSERT INTO helqix_users(id,name,email,organization,password_salt,password_hash,created_at) VALUES(?,?,?,?,?,?,?)",
                (user_id, name, email, organization, salt, password_hash, now))
            create_session(database, user_id, response)
            row = database.execute("SELECT * FROM helqix_users WHERE id=?", (user_id,)).fetchone()
    except sqlite3.IntegrityError as exc:
        raise HTTPException(status_code=409, detail="An account with this email already exists") from exc
    return {"user": user_public(row)}


@app.post("/v1/auth/local-workspace")
async def open_local_workspace(request: Request, response: Response) -> dict[str, Any]:
    """Open the single-user local workspace without exposing a public login bypass.

    This entry point is deliberately restricted to loopback clients and is
    disabled when API-key mode is configured. Project and resource routes still
    require the short-lived, HttpOnly session created here.
    """
    host = request.client.host if request.client else ""
    try:
        is_loopback = ipaddress.ip_address(host).is_loopback
    except ValueError:
        is_loopback = False
    if not is_loopback:
        raise HTTPException(status_code=403, detail="Local workspace entry is only available from this computer")

    email = "local-workspace@helqix.invalid"
    with _local_workspace_lock, open_database() as database:
        row = database.execute("SELECT * FROM helqix_users WHERE email=?", (email,)).fetchone()
        if row is None:
            user_id, now = str(uuid.uuid4()), time.time()
            # Credentials are random and never returned or accepted by an API.
            salt, password_hash = secrets.token_bytes(16), secrets.token_bytes(32)
            database.execute(
                "INSERT INTO helqix_users(id,name,email,organization,password_salt,password_hash,created_at) VALUES(?,?,?,?,?,?,?)",
                (user_id, "Local workspace", email, "This computer", salt, password_hash, now),
            )
            row = database.execute("SELECT * FROM helqix_users WHERE id=?", (user_id,)).fetchone()
        create_session(database, row["id"], response)
    return {"user": user_public(row)}


@app.post("/v1/auth/login")
async def sign_in(body: LoginRequest, response: Response) -> dict[str, Any]:
    email = body.email.strip().lower()
    with open_database() as database:
        row = database.execute("SELECT * FROM helqix_users WHERE email=?", (email,)).fetchone()
        salt = row["password_salt"] if row else b"\0" * 16
        actual = await asyncio.to_thread(hashlib.pbkdf2_hmac, "sha256", body.password.encode(), salt, PASSWORD_ITERATIONS)
        valid = row is not None and hmac.compare_digest(actual, row["password_hash"])
        if not valid:
            raise HTTPException(status_code=401, detail="Email or password is incorrect")
        create_session(database, row["id"], response)
    return {"user": user_public(row)}


@app.get("/v1/auth/me")
async def session_user(session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    return {"user": user_public(get_current_user(session_token))}


@app.post("/v1/auth/logout", status_code=204)
async def sign_out(response: Response, session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> Response:
    if session_token:
        digest = hashlib.sha256(session_token.encode()).hexdigest()
        with open_database() as database:
            database.execute("DELETE FROM helqix_sessions WHERE token_hash=?", (digest,))
    response.delete_cookie(COOKIE_NAME, path="/", secure=COOKIE_SECURE, httponly=True, samesite="lax")
    response.status_code = 204
    return response


@app.post("/v1/intelligence/intent/parse")
async def parse_intent(body: IntentParseRequest, session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    get_current_user(session_token)
    try:
        result = parse_engineering_intent(body.intent, body.domain)
        result["ai_review_status"] = ai_review_provider_status()
        if body.assisted:
            result["ai_review"] = await asyncio.to_thread(review_engineering_intent_with_ai, body.intent)
        return result
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc


@app.get("/v1/libraries/component-types")
async def list_component_types(domain: str | None = Query(default=None, max_length=120),
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    """Return versioned type classifications for the authenticated project builder."""
    get_current_user(session_token)
    return component_type_catalog(domain)


@app.get("/v1/{resource}")
async def list_resources(resource: str, request: Request, project_id: str | None = Query(default=None), limit: int = Query(default=50, ge=1, le=100), session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    resource = validate_resource(resource)
    user = get_current_user(session_token)
    query = "SELECT * FROM helqix_resources WHERE user_id=? AND resource_type=?"
    values: list[Any] = [user["id"], resource]
    if project_id:
        query += " AND project_id=?"; values.append(project_id)
    query += " ORDER BY created_at DESC,id DESC LIMIT ?"; values.append(limit)
    with open_database() as database:
        rows = database.execute(query, values).fetchall()
    return {"data": [resource_record(row) for row in rows], "has_more": False}


@app.get("/v1/projects/{project_id}/evidence/export")
async def export_project_evidence(project_id: str,
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> Response:
    user = get_current_user(session_token)
    project = owned_project(project_id, user["id"])
    resources: dict[str, list[dict[str, Any]]] = {}
    maximum_records = 10_000
    with open_database() as database:
        for resource_type in sorted(ALLOWED_RESOURCES - {"projects"}):
            rows = database.execute(
                "SELECT * FROM helqix_resources WHERE user_id=? AND resource_type=? AND project_id=? ORDER BY created_at ASC,id ASC LIMIT ?",
                (user["id"], resource_type, project["id"], maximum_records + 1),
            ).fetchall()
            if len(rows) > maximum_records:
                raise HTTPException(status_code=413, detail=f"Project has more than {maximum_records} {resource_type} records; export is refused rather than silently truncated")
            resources[resource_type] = [resource_record(row) for row in rows]
        integration_rows = database.execute(
            "SELECT * FROM helqix_integration_runs WHERE user_id=? AND project_id=? ORDER BY created_at ASC,id ASC LIMIT ?",
            (user["id"], project["id"], maximum_records + 1),
        ).fetchall()
    if len(integration_rows) > maximum_records:
        raise HTTPException(status_code=413, detail=f"Project has more than {maximum_records} integration runs; export is refused rather than silently truncated")

    bundle: dict[str, Any] = {
        "schema": "helqix.project-evidence-bundle.v1",
        "export_id": str(uuid.uuid4()),
        "exported_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "project": resource_record(project),
        "resources": resources,
        "integration_runs": [{
            "id": row["id"], "created_at": row["created_at"], "tool": row["tool"],
            "operation": row["operation"], "inputs": json.loads(row["inputs_json"]),
            "result": json.loads(row["result_json"]), "provenance": row["provenance"],
            "project_id": row["project_id"],
            "project_snapshot": json.loads(row["project_snapshot_json"]) if row["project_snapshot_json"] else None,
        } for row in integration_rows],
        "assurance": {
            "status": "traceability_only",
            "verification_established": False,
            "custom_drone_flight_simulation_performed": False,
            "limitations": [
                "This file records saved inputs and tool outputs; it is not an engineering certificate.",
                "A Blender design draft is visual geometry and is not CAD, structural, or flight validated.",
                "A PX4 stock-airframe run, if present, is not evidence for the project's custom drone.",
            ],
        },
    }
    canonical = json.dumps(bundle, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
    bundle["bundle_sha256"] = hashlib.sha256(canonical).hexdigest()
    contents = json.dumps(bundle, ensure_ascii=False, indent=2, allow_nan=False)
    return Response(content=contents, media_type="application/json",
        headers={"Content-Disposition": f'attachment; filename="helqix-evidence-{project["id"]}.json"',
                 "X-HELQIX-Bundle-SHA256": bundle["bundle_sha256"]})


@app.get("/v1/projects/{project_id}/activity")
async def project_activity(project_id: str, after_seq: int = Query(default=0, ge=0),
        limit: int = Query(default=50, ge=1, le=100),
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    """Return safe, append-only audit events for a project owned by the signed-in user."""
    user = get_current_user(session_token)
    project = owned_project(project_id, user["id"])
    with open_database() as database:
        rows = database.execute(
            "SELECT seq,id,created_at,action,subject_type,subject_id,outcome,safe_details_json "
            "FROM helqix_audit_events WHERE actor_user_id=? AND project_id=? AND seq>? ORDER BY seq ASC LIMIT ?",
            (user["id"], project["id"], after_seq, limit + 1),
        ).fetchall()
    has_more = len(rows) > limit
    events = [{"seq": row["seq"], "id": row["id"], "created_at": row["created_at"],
               "action": row["action"], "subject_type": row["subject_type"],
               "subject_id": row["subject_id"], "outcome": row["outcome"],
               "details": json.loads(row["safe_details_json"])} for row in rows[:limit]]
    return {"events": events, "next_cursor": events[-1]["seq"] if events else after_seq,
            "has_more": has_more, "project_id": project["id"]}


def _proofloop_context(project: sqlite3.Row, user_id: str) -> dict[str, Any]:
    project_payload = json.loads(project["payload_json"])
    reality = project_payload.get("engineering_reality") if isinstance(project_payload, dict) else None
    snapshot = project_snapshot(project)
    current_digest = (((snapshot.get("models") or {}).get("engineering_reality") or {}).get("sha256"))
    with open_database() as database:
        run_rows = database.execute(
            "SELECT * FROM helqix_integration_runs WHERE user_id=? AND project_id=? ORDER BY created_at DESC,id DESC LIMIT 100",
            (user_id, project["id"]),
        ).fetchall()
        verification_rows = database.execute(
            "SELECT * FROM helqix_resources WHERE user_id=? AND project_id=? AND resource_type='verifications' ORDER BY created_at DESC,id DESC LIMIT 500",
            (user_id, project["id"]),
        ).fetchall()
    runs = [{"id": row["id"], "created_at": row["created_at"], "tool": row["tool"],
             "operation": row["operation"], "inputs": json.loads(row["inputs_json"]),
             "result": json.loads(row["result_json"]),
             "project_snapshot": json.loads(row["project_snapshot_json"]) if row["project_snapshot_json"] else None}
            for row in run_rows]
    checks = [json.loads(row["payload_json"]) for row in verification_rows]
    recommendation = recommend_next_step(project_id=project["id"], reality=reality,
                                         current_model_sha256=current_digest, runs=runs, checks=checks)
    recommendation["generated_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    recommendation["project_revision"] = project["version"]
    recommendation["model_sha256"] = current_digest
    return recommendation


@app.get("/v1/projects/{project_id}/proofloop/recommendation")
async def proofloop_recommendation(project_id: str,
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    """Return a transparent, evidence-backed next engineering proof obligation."""
    user = get_current_user(session_token)
    project = owned_project(project_id, user["id"])
    recommendation = _proofloop_context(project, user["id"])
    return {"recommendation": recommendation}


@app.post("/v1/projects/{project_id}/proofloop/record", status_code=201)
async def record_proofloop_recommendation(project_id: str, body: ProofLoopRecordRequest,
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    """Record that the user added the current recommendation to the project plan."""
    user = get_current_user(session_token)
    project = owned_project(project_id, user["id"])
    recommendation = _proofloop_context(project, user["id"])
    if recommendation["action_id"] != body.action_id:
        raise HTTPException(status_code=409, detail="The recommendation changed with the project evidence; review the refreshed next step.")
    current_digest = (((project_snapshot(project).get("models") or {}).get("engineering_reality") or {}).get("sha256"))
    with open_database() as database:
        existing = database.execute(
            "SELECT * FROM helqix_resources WHERE user_id=? AND project_id=? AND resource_type='proofloops' ORDER BY created_at DESC,id DESC LIMIT 100",
            (user["id"], project["id"]),
        ).fetchall()
    for row in existing:
        previous = json.loads(row["payload_json"])
        if (previous.get("action_id") == body.action_id
                and previous.get("model_sha256") == current_digest
                and previous.get("evidence_refs") == recommendation.get("evidence_refs")):
            return {"data": resource_record(row), "already_recorded": True}
    payload = {"schema": "helqix.proofloop-plan.v1", "status": "planned",
               "action_id": recommendation["action_id"], "title": recommendation["title"],
               "reason": recommendation["reason"], "detail": recommendation["detail"],
               "destination": recommendation["destination"], "priority": recommendation["priority"],
               "evidence_refs": recommendation["evidence_refs"], "limitations": recommendation["limitations"],
               "model_sha256": current_digest, "project_revision": project["version"],
               "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    return {"data": persist_project_resource("proofloops", payload, user["id"], project["id"]),
            "already_recorded": False}


@app.post("/v1/{resource}/query")
async def search_resources(resource: str, request: Request, session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    resource = validate_resource(resource)
    user = get_current_user(session_token)
    body = await request.json()
    contains = body.get("payload_contains", body.get("payloadContains", {}))
    if not isinstance(contains, dict):
        raise HTTPException(status_code=422, detail="payload_contains must be an object")
    project_id = body.get("project_id", body.get("projectId"))
    limit = body.get("limit", 50)
    if not isinstance(limit, int) or isinstance(limit, bool) or not 1 <= limit <= 100:
        raise HTTPException(status_code=422, detail="limit must be between 1 and 100")
    with open_database() as database:
        rows = database.execute("SELECT * FROM helqix_resources WHERE user_id=? AND resource_type=? ORDER BY created_at DESC LIMIT ?", (user["id"], resource, limit)).fetchall()
    matches = []
    for row in rows:
        if project_id and row["project_id"] != project_id:
            continue
        payload = json.loads(row["payload_json"])
        if all(payload.get(key) == value for key, value in contains.items()):
            matches.append(resource_record(row))
    return {"data": matches, "has_more": False}


@app.post("/v1/{resource}", status_code=201)
async def create_resource(resource: str, body: ResourceCreate, response: Response, request: Request,
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    resource = validate_resource(resource)
    user = get_current_user(session_token)
    body.payload = validate_domain_payload(resource, body.payload)
    user_id, record_id, now = user["id"], body.id or str(uuid.uuid4()), time.time()
    try:
        record_id = str(uuid.UUID(record_id))
    except ValueError as exc:
        raise HTTPException(status_code=422, detail="resource id must be a UUID") from exc
    if body.project_id:
        with open_database() as database:
            parent = database.execute("SELECT 1 FROM helqix_resources WHERE id=? AND resource_type='projects' AND user_id=?", (body.project_id, user_id)).fetchone()
        if not parent:
            raise HTTPException(status_code=404, detail="Project not found")
    payload_json = serialize_payload(body.payload)
    key = request.headers.get("idempotency-key")
    with open_database() as database:
        if key:
            if len(key) > 200 or not key.strip() or key.strip() != key:
                raise HTTPException(status_code=422, detail="Idempotency-Key must be 1-200 trimmed characters")
            fingerprint = hashlib.sha256(json.dumps({"payload": body.payload, "project_id": body.project_id, "id": body.id}, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
            existing = database.execute("SELECT fingerprint,resource_id FROM helqix_idempotency WHERE user_id=? AND resource_type=? AND request_key=?", (user_id, resource, key)).fetchone()
            if existing:
                if existing["fingerprint"] != fingerprint:
                    raise HTTPException(status_code=409, detail="Idempotency-Key was reused with a different request")
                row = database.execute("SELECT * FROM helqix_resources WHERE id=? AND user_id=?", (existing["resource_id"], user_id)).fetchone()
                return {"data": resource_record(row)}
        database.execute("INSERT INTO helqix_resources VALUES(?,?,?,?,?,1,?,?)", (record_id, resource, user_id, body.project_id, payload_json, now, now))
        if key:
            database.execute("INSERT INTO helqix_idempotency VALUES(?,?,?,?,?,?)", (user_id, resource, key, fingerprint, record_id, now))
        row = database.execute("SELECT * FROM helqix_resources WHERE id=?", (record_id,)).fetchone()
        audit_project_id = record_id if resource == "projects" else body.project_id
        _append_audit_event(database, user_id, audit_project_id, "resource.created", resource, record_id,
                            safe_details={"version": 1})
    response.headers["ETag"] = '"1"'
    return {"data": resource_record(row)}


@app.get("/v1/{resource}/{resource_id}")
async def get_resource(resource: str, resource_id: str, session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    resource = validate_resource(resource); user = get_current_user(session_token)
    with open_database() as database:
        row = database.execute("SELECT * FROM helqix_resources WHERE id=? AND resource_type=? AND user_id=?", (resource_id, resource, user["id"])).fetchone()
    if row is None: raise HTTPException(status_code=404, detail="Resource not found")
    return {"data": resource_record(row)}


@app.put("/v1/{resource}/{resource_id}")
async def replace_resource(resource: str, resource_id: str, body: ResourceReplace, request: Request, response: Response,
        session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> dict[str, Any]:
    resource = validate_resource(resource); user = get_current_user(session_token)
    body.payload = validate_domain_payload(resource, body.payload)
    match = request.headers.get("if-match", "")
    if not re.fullmatch(r'"[1-9][0-9]*"', match):
        raise HTTPException(status_code=428, detail="A strong If-Match version is required")
    expected = int(match[1:-1]); now = time.time()
    if body.project_id:
        with open_database() as database:
            parent = database.execute("SELECT 1 FROM helqix_resources WHERE id=? AND resource_type='projects' AND user_id=?", (body.project_id, user["id"])).fetchone()
        if not parent:
            raise HTTPException(status_code=404, detail="Project not found")
    with open_database() as database:
        result = database.execute("UPDATE helqix_resources SET payload_json=?,project_id=COALESCE(?,project_id),version=version+1,updated_at=? WHERE id=? AND resource_type=? AND user_id=? AND version=?",
            (serialize_payload(body.payload), body.project_id, now, resource_id, resource, user["id"], expected))
        if result.rowcount == 0:
            exists = database.execute("SELECT version FROM helqix_resources WHERE id=? AND resource_type=? AND user_id=?", (resource_id, resource, user["id"])).fetchone()
            if exists is None: raise HTTPException(status_code=404, detail="Resource not found")
            raise HTTPException(status_code=412, detail="Resource changed; reload it before saving")
        row = database.execute("SELECT * FROM helqix_resources WHERE id=?", (resource_id,)).fetchone()
        audit_project_id = row["id"] if resource == "projects" else row["project_id"]
        _append_audit_event(database, user["id"], audit_project_id, "resource.updated", resource, resource_id,
                            safe_details={"version": row["version"]})
    response.headers["ETag"] = f'"{row["version"]}"'
    return {"data": resource_record(row)}


@app.delete("/v1/{resource}/{resource_id}", status_code=204)
async def delete_resource(resource: str, resource_id: str, request: Request, session_token: str | None = Cookie(default=None, alias=COOKIE_NAME)) -> Response:
    resource = validate_resource(resource); user = get_current_user(session_token)
    match = request.headers.get("if-match", "")
    if not re.fullmatch(r'"[1-9][0-9]*"', match):
        raise HTTPException(status_code=428, detail="A strong If-Match version is required")
    with open_database() as database:
        existing = database.execute("SELECT project_id,version FROM helqix_resources WHERE id=? AND resource_type=? AND user_id=?",
                                     (resource_id, resource, user["id"])).fetchone()
        deleted = database.execute("DELETE FROM helqix_resources WHERE id=? AND resource_type=? AND user_id=? AND version=?", (resource_id, resource, user["id"], int(match[1:-1]))).rowcount
        if deleted == 0: raise HTTPException(status_code=404, detail="Resource not found or version changed")
        if existing:
            audit_project_id = resource_id if resource == "projects" else existing["project_id"]
            _append_audit_event(database, user["id"], audit_project_id, "resource.deleted", resource, resource_id,
                                safe_details={"version": existing["version"]})
    return Response(status_code=204)
