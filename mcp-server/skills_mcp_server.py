#!/usr/bin/env python3
"""skills-mcp — serves SkillBasin skills to MCP clients that have no
directory loader of their own (OpenClaw, custom agents, anything speaking MCP).

Zero dependencies: raw JSON-RPC 2.0 over stdio, or streamable HTTP (POST /mcp).

Three fixed tools (NOT tool-per-skill — keeps client context small):
  skills_list()        -> name + description index
  skill_load(name)     -> full SKILL.md body
  skill_search(query)  -> search across names/descriptions/bodies

Two catalog modes:
  serve-dir DIR
      Serve every SKILL.md found recursively under DIR (plain directory).
  serve-basin BASIN --machine ID [--tool KEY]
      Serve ONLY the versions pinned to KEY on machine ID, straight from the
      basin's versioned store (skills/<name>/versions/<version>/SKILL.md).
      KEY defaults to the "tool" field of machines/<ID>/mcp-serve.json.

Transports: --transport stdio (default) | http [--port N, 0 = ephemeral].
The HTTP listener prints "SKILLS_MCP_LISTENING port=N" once ready.

Every request rescans the source, so a git pull / pin change is picked up
without a restart.
"""
from __future__ import annotations

import argparse
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

PROTOCOL_VERSION = "2024-11-05"
SERVER_INFO = {"name": "skills-mcp", "version": "0.2.0"}

TOOLS = [
    {
        "name": "skills_list",
        "description": "List all available skills (name + description index). "
                       "Call this first to discover what skills exist.",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "skill_load",
        "description": "Load the full instructions (SKILL.md body) of one skill by name. "
                       "Call when a task matches a skill's description.",
        "inputSchema": {
            "type": "object",
            "properties": {"name": {"type": "string", "description": "Skill name from skills_list"}},
            "required": ["name"],
        },
    },
    {
        "name": "skill_search",
        "description": "Full-text search across skill names, descriptions and bodies.",
        "inputSchema": {
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"],
        },
    },
]


# ── frontmatter ────────────────────────────────────────────────────────────

def parse_frontmatter(text: str) -> dict:
    """Top-level `name:`/`description:` from YAML frontmatter, including
    multiline values: folded (>, >-), literal (|, |-), and plain continuation
    lines. Not a YAML parser — just the subset SKILL.md files actually use."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return {}
    try:
        end = next(i for i in range(1, len(lines)) if lines[i].strip() == "---")
    except StopIteration:
        return {}

    meta: dict[str, str] = {}
    i = 1
    while i < end:
        line = lines[i]
        stripped = line.strip()
        key, sep, rest = stripped.partition(":")
        if not sep or line[:1] in (" ", "\t") or not key or " " in key:
            i += 1
            continue
        rest = rest.strip()
        if rest in (">", ">-", ">+", "|", "|-", "|+"):
            # Block scalar: collect the indented block that follows.
            block: list[str] = []
            i += 1
            while i < end and (not lines[i].strip() or lines[i][:1] in (" ", "\t")):
                block.append(lines[i].strip())
                i += 1
            joiner = " " if rest.startswith(">") else "\n"
            meta[key] = joiner.join(b for b in block if b)
            continue
        # Plain value, possibly followed by indented continuation lines.
        value = [rest]
        i += 1
        while i < end and lines[i][:1] in (" ", "\t") and lines[i].strip():
            nxt = lines[i].strip()
            k2, s2, _ = nxt.partition(":")
            if s2 and " " not in k2 and k2.isidentifier():
                break  # looks like a nested key, not a continuation
            value.append(nxt)
            i += 1
        joined = " ".join(v for v in value if v)
        if (joined.startswith('"') and joined.endswith('"')) or (
            joined.startswith("'") and joined.endswith("'")
        ):
            joined = joined[1:-1]
        meta[key] = joined
    return {k: v for k, v in meta.items() if k in ("name", "description")}


def read_skill_md(path: Path) -> str | None:
    try:
        # utf-8-sig: tolerate the BOM that Windows editors and PowerShell 5.1's
        # `Set-Content -Encoding utf8` prepend; reads plain UTF-8 unchanged.
        return path.read_text(encoding="utf-8-sig", errors="replace")
    except OSError:
        return None


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError):
        return None


# ── catalogs ───────────────────────────────────────────────────────────────

class DirCatalog:
    """Plain directory mode: every SKILL.md under root (recursive) is a skill."""

    def __init__(self, root: Path):
        self.root = Path(root)

    def _entries(self) -> list[dict]:
        out = []
        if not self.root.exists():
            return out
        for skill_md in sorted(self.root.glob("**/SKILL.md")):
            text = read_skill_md(skill_md)
            if text is None:
                continue
            meta = parse_frontmatter(text)
            out.append({
                "name": meta.get("name") or skill_md.parent.name,
                "description": meta.get("description", "(no description)"),
                "path": skill_md,
            })
        return out

    def list(self) -> list[dict]:
        return [{"name": e["name"], "description": e["description"]}
                for e in self._entries()]

    def load(self, name: str) -> str | None:
        for e in self._entries():
            if e["name"] == name:
                return read_skill_md(e["path"])
        return None


class BasinCatalog:
    """Pin-filtered mode: serve exactly the versions pinned to one tool on one
    machine, from the basin's versioned store. The pins lockfile is the single
    source of truth — nothing outside it is visible to the client."""

    def __init__(self, basin: Path, machine: str, tool: str):
        self.basin = Path(basin)
        self.machine = machine
        self.tool = tool

    def _pinned(self) -> list[dict]:
        pins = read_json(self.basin / "machines" / self.machine / "pins.json")
        if pins is None:
            return []
        out = []
        for entry in pins.get("pins", []):
            target = entry.get("targets", {}).get(self.tool)
            if not target or not target.get("enabled", True):
                continue
            skill, version = entry.get("skill"), entry.get("version")
            if not skill or not version:
                continue
            skill_md = (self.basin / "skills" / skill / "versions"
                        / version / "SKILL.md")
            text = read_skill_md(skill_md)
            if text is None:
                continue  # pin points at a version the basin doesn't have
            meta = parse_frontmatter(text)
            out.append({
                "name": meta.get("name") or skill,
                "version": version,
                "description": meta.get("description", "(no description)"),
                "path": skill_md,
            })
        return sorted(out, key=lambda e: e["name"])

    def list(self) -> list[dict]:
        return [{"name": e["name"], "version": e["version"],
                 "description": e["description"]} for e in self._pinned()]

    def load(self, name: str) -> str | None:
        for e in self._pinned():
            if e["name"] == name:
                return read_skill_md(e["path"])
        return None


def read_serve_config(basin: Path, machine: str) -> str | None:
    """machines/<id>/mcp-serve.json -> which tool this machine's server
    speaks for. CLI --tool overrides it."""
    config = read_json(Path(basin) / "machines" / machine / "mcp-serve.json")
    return config.get("tool") if isinstance(config, dict) else None


# ── JSON-RPC core ──────────────────────────────────────────────────────────

def tool_result(text: str) -> dict:
    return {"content": [{"type": "text", "text": text}], "isError": False}


def tool_error(text: str) -> dict:
    return {"content": [{"type": "text", "text": text}], "isError": True}


def handle_tool_call(catalog, name: str, args: dict) -> dict:
    if name == "skills_list":
        skills = catalog.list()
        if not skills:
            return tool_result("No skills available.")
        lines = []
        for s in skills:
            tag = f" (v{s['version']})" if s.get("version") else ""
            lines.append(f"- {s['name']}{tag}: {s['description']}")
        return tool_result(f"{len(skills)} skill available:\n" + "\n".join(lines))
    if name == "skill_load":
        want = args.get("name", "")
        body = catalog.load(want)
        if body is None:
            return tool_error(
                f"Skill not found: {want!r}. Use skills_list to see available names.")
        return tool_result(body)
    if name == "skill_search":
        q = args.get("query", "").lower()
        hits = []
        for s in catalog.list():
            body = catalog.load(s["name"]) or ""
            if (q in s["name"].lower() or q in s["description"].lower()
                    or q in body.lower()):
                hits.append(f"- {s['name']}: {s['description']}")
        return tool_result("\n".join(hits) if hits else f"No skills match {q!r}")
    return tool_error(f"Unknown tool: {name}")


def handle_request(catalog, req: dict):
    method = req.get("method", "")
    rid = req.get("id")
    if method == "initialize":
        return {"jsonrpc": "2.0", "id": rid, "result": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": SERVER_INFO,
        }}
    if method == "notifications/initialized":
        return None
    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": rid, "result": {"tools": TOOLS}}
    if method == "tools/call":
        params = req.get("params", {})
        result = handle_tool_call(catalog, params.get("name", ""),
                                  params.get("arguments", {}))
        return {"jsonrpc": "2.0", "id": rid, "result": result}
    if method == "ping":
        return {"jsonrpc": "2.0", "id": rid, "result": {}}
    if rid is not None:
        return {"jsonrpc": "2.0", "id": rid,
                "error": {"code": -32601, "message": f"Method not found: {method}"}}
    return None


# ── transports ─────────────────────────────────────────────────────────────

def run_stdio(catalog) -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        resp = handle_request(catalog, req)
        if resp is not None:
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()


def run_http(catalog, port: int) -> None:
    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):  # noqa: N802 (stdlib naming)
            if self.path.rstrip("/") != "/mcp":
                self.send_error(404)
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                req = json.loads(self.rfile.read(length).decode("utf-8"))
            except (ValueError, json.JSONDecodeError):
                self.send_error(400, "invalid JSON-RPC payload")
                return
            resp = handle_request(catalog, req)
            if resp is None:  # notification — MCP spec: 202, no body
                self.send_response(202)
                self.end_headers()
                return
            body = json.dumps(resp).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, fmt, *args):  # keep stdout clean for the port line
            sys.stderr.write("[http] " + fmt % args + "\n")

    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    print(f"SKILLS_MCP_LISTENING port={server.server_address[1]}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


# ── CLI ────────────────────────────────────────────────────────────────────

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="skills-mcp", description=__doc__)
    sub = parser.add_subparsers(dest="mode", required=True)

    p_dir = sub.add_parser("serve-dir", help="serve a plain skills directory")
    p_dir.add_argument("dir", type=Path)

    p_basin = sub.add_parser("serve-basin",
                             help="serve only the versions pinned to one tool")
    p_basin.add_argument("basin", type=Path)
    p_basin.add_argument("--machine", required=True)
    p_basin.add_argument("--tool", default=None,
                         help="defaults to machines/<id>/mcp-serve.json")

    for p in (p_dir, p_basin):
        p.add_argument("--transport", choices=["stdio", "http"], default="stdio")
        p.add_argument("--port", type=int, default=8750)

    args = parser.parse_args(argv)

    if args.mode == "serve-dir":
        catalog = DirCatalog(args.dir)
    else:
        tool = args.tool or read_serve_config(args.basin, args.machine)
        if not tool:
            parser.error(
                "no tool given and machines/"
                f"{args.machine}/mcp-serve.json has none — pass --tool")
        catalog = BasinCatalog(args.basin, args.machine, tool)

    if args.transport == "http":
        run_http(catalog, args.port)
    else:
        run_stdio(catalog)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
