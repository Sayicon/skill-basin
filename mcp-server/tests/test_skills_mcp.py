"""skills-mcp repo tests.

Spike'ın test_harness.py'ının ürünleşmiş hâli: gerçek kullanıcı dizinine değil
fixture'lara karşı koşar, CI'da `python -m unittest` ile çalışır.

Kapsam:
- frontmatter parser (tek satır + çok satırlı YAML: folded `>-`, literal `|`)
- serve-dir modu (düz skills dizini)
- serve-basin modu (pins.json filtresi: yalnız o ajana pinli versiyonlar)
- stdio JSON-RPC round-trip (subprocess)
- streamable-http round-trip (POST /mcp)
- hata yolları (olmayan skill, pinsiz tool)
"""
import json
import os
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.request
from pathlib import Path

SERVER = Path(__file__).resolve().parent.parent / "skills_mcp_server.py"
sys.path.insert(0, str(SERVER.parent))

import skills_mcp_server as srv  # noqa: E402


def write_skill(root: Path, name: str, description: str, body: str = "gövde") -> None:
    d = root / name
    d.mkdir(parents=True, exist_ok=True)
    (d / "SKILL.md").write_text(
        f"---\nname: {name}\ndescription: {description}\n---\n\n{body}\n",
        encoding="utf-8",
    )


def make_basin(root: Path) -> Path:
    """demo@1.0.0 + demo@2.0.0 + solo@1.0.0; m1 makinesinde hermes_agent'a
    yalnız demo@1.0.0 pinli. solo hiçbir tool'a pinli değil."""
    basin = root / "basin"
    for skill, version, body in [
        ("demo", "1.0.0", "demo v1 body"),
        ("demo", "2.0.0", "demo v2 body"),
        ("solo", "1.0.0", "solo body"),
    ]:
        d = basin / "skills" / skill / "versions" / version
        d.mkdir(parents=True)
        (d / "SKILL.md").write_text(
            f"---\nname: {skill}\ndescription: {skill} {version} açıklaması\n---\n\n{body}\n",
            encoding="utf-8",
        )
    m = basin / "machines" / "m1"
    m.mkdir(parents=True)
    (m / "pins.json").write_text(json.dumps({
        "machine": "m1",
        "pins": [
            {"skill": "demo", "version": "1.0.0",
             "targets": {"hermes_agent": {"enabled": True, "strategy": "auto"}}},
            {"skill": "demo", "version": "2.0.0",
             "targets": {"cursor": {"enabled": True, "strategy": "auto"}}},
        ],
    }), encoding="utf-8")
    (m / "mcp-serve.json").write_text(json.dumps({"tool": "hermes_agent"}),
                                      encoding="utf-8")
    return basin


class FrontmatterTests(unittest.TestCase):
    def test_single_line(self):
        meta = srv.parse_frontmatter("---\nname: a\ndescription: tek satır\n---\nx")
        self.assertEqual(meta["description"], "tek satır")

    def test_folded_multiline(self):
        text = ("---\n"
                "name: video-analysis\n"
                "description: >-\n"
                "  v3 — video analiz + Whisper transkript\n"
                "  + süre bazlı otomatik model seçimi\n"
                "---\n\nbody\n")
        meta = srv.parse_frontmatter(text)
        self.assertEqual(
            meta["description"],
            "v3 — video analiz + Whisper transkript + süre bazlı otomatik model seçimi",
        )

    def test_literal_multiline(self):
        text = ("---\n"
                "name: x\n"
                "description: |\n"
                "  ilk satır\n"
                "  ikinci satır\n"
                "---\nbody\n")
        meta = srv.parse_frontmatter(text)
        self.assertEqual(meta["description"], "ilk satır\nikinci satır")

    def test_plain_continuation(self):
        text = ("---\n"
                "name: x\n"
                "description: başlangıç\n"
                "  devam satırı\n"
                "tags: [a, b]\n"
                "---\nbody\n")
        meta = srv.parse_frontmatter(text)
        self.assertEqual(meta["description"], "başlangıç devam satırı")

    def test_quoted(self):
        meta = srv.parse_frontmatter('---\nname: a\ndescription: "tırnaklı: değer"\n---\nx')
        self.assertEqual(meta["description"], "tırnaklı: değer")


class DirModeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        write_skill(self.root, "alpha", "ilk skill")
        write_skill(self.root / "nested", "beta", "kategorili skill")

    def tearDown(self):
        self.tmp.cleanup()

    def test_scan_recursive_and_load(self):
        cat = srv.DirCatalog(self.root)
        names = sorted(s["name"] for s in cat.list())
        self.assertEqual(names, ["alpha", "beta"])
        body = cat.load("beta")
        self.assertIn("kategorili skill", body)

    def test_load_missing_is_none(self):
        cat = srv.DirCatalog(self.root)
        self.assertIsNone(cat.load("yok-boyle-skill"))


class BasinModeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.basin = make_basin(Path(self.tmp.name))

    def tearDown(self):
        self.tmp.cleanup()

    def test_only_pinned_versions_served(self):
        cat = srv.BasinCatalog(self.basin, "m1", "hermes_agent")
        skills = cat.list()
        self.assertEqual([s["name"] for s in skills], ["demo"])
        self.assertEqual(skills[0]["version"], "1.0.0")
        body = cat.load("demo")
        self.assertIn("demo v1 body", body)
        self.assertNotIn("v2", body)

    def test_unpinned_skill_not_loadable(self):
        cat = srv.BasinCatalog(self.basin, "m1", "hermes_agent")
        self.assertIsNone(cat.load("solo"))

    def test_other_tool_sees_its_own_pin(self):
        cat = srv.BasinCatalog(self.basin, "m1", "cursor")
        skills = cat.list()
        self.assertEqual([(s["name"], s["version"]) for s in skills],
                         [("demo", "2.0.0")])

    def test_tool_without_pins_is_empty(self):
        cat = srv.BasinCatalog(self.basin, "m1", "claude_code")
        self.assertEqual(cat.list(), [])

    def test_tool_read_from_mcp_serve_json(self):
        tool = srv.read_serve_config(self.basin, "m1")
        self.assertEqual(tool, "hermes_agent")

    def test_bom_prefixed_json_files_still_parse(self):
        # PowerShell 5.1's `Set-Content -Encoding utf8` writes a UTF-8 BOM;
        # a Windows user hand-writing pins.json/mcp-serve.json hits this.
        m = self.basin / "machines" / "m1"
        for fname in ("pins.json", "mcp-serve.json"):
            p = m / fname
            p.write_bytes(b"\xef\xbb\xbf" + p.read_bytes())
        self.assertEqual(srv.read_serve_config(self.basin, "m1"), "hermes_agent")
        cat = srv.BasinCatalog(self.basin, "m1", "hermes_agent")
        self.assertEqual([s["name"] for s in cat.list()], ["demo"])

    def test_disabled_pin_target_not_served(self):
        pins_path = self.basin / "machines" / "m1" / "pins.json"
        pins = json.loads(pins_path.read_text(encoding="utf-8"))
        pins["pins"][0]["targets"]["hermes_agent"]["enabled"] = False
        pins_path.write_text(json.dumps(pins), encoding="utf-8")
        cat = srv.BasinCatalog(self.basin, "m1", "hermes_agent")
        self.assertEqual(cat.list(), [])


class HandleRequestTests(unittest.TestCase):
    def test_non_object_payload_is_rejected_not_crashed(self):
        cat = srv.DirCatalog(tempfile.gettempdir())
        for bad in ([1, 2, 3], "hello", 42, None):
            resp = srv.handle_request(cat, bad)
            self.assertIsNotNone(resp, f"{bad!r} must get an error response")
            self.assertEqual(resp["error"]["code"], -32600)

    def test_object_payload_still_works(self):
        cat = srv.DirCatalog(tempfile.gettempdir())
        resp = srv.handle_request(cat, {"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
        self.assertIn("result", resp)


def rpc_lines(proc, payload):
    proc.stdin.write(json.dumps(payload) + "\n")
    proc.stdin.flush()
    if "id" not in payload:
        return None
    return json.loads(proc.stdout.readline())


class StdioRoundTripTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        write_skill(self.root, "alpha", "ilk skill", body="alpha gövdesi")
        self.proc = subprocess.Popen(
            [sys.executable, str(SERVER), "serve-dir", str(self.root)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            text=True, encoding="utf-8",
        )

    def tearDown(self):
        self.proc.stdin.close()
        self.proc.stdout.close()
        self.proc.terminate()
        self.proc.wait(timeout=10)
        self.tmp.cleanup()

    def test_initialize_list_call_and_error_path(self):
        r = rpc_lines(self.proc, {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                       "clientInfo": {"name": "t", "version": "0"}}})
        self.assertEqual(r["result"]["serverInfo"]["name"], "skills-mcp")
        rpc_lines(self.proc, {"jsonrpc": "2.0", "method": "notifications/initialized"})

        r = rpc_lines(self.proc, {"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
        names = [t["name"] for t in r["result"]["tools"]]
        self.assertEqual(names, ["skills_list", "skill_load", "skill_search"])

        r = rpc_lines(self.proc, {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                                  "params": {"name": "skill_load",
                                             "arguments": {"name": "alpha"}}})
        self.assertFalse(r["result"]["isError"])
        self.assertIn("alpha gövdesi", r["result"]["content"][0]["text"])

        r = rpc_lines(self.proc, {"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                                  "params": {"name": "skill_load",
                                             "arguments": {"name": "yok"}}})
        self.assertTrue(r["result"]["isError"])


class HttpRoundTripTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.basin = make_basin(Path(self.tmp.name))
        self.proc = subprocess.Popen(
            [sys.executable, str(SERVER), "serve-basin", str(self.basin),
             "--machine", "m1", "--transport", "http", "--port", "0"],
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, encoding="utf-8",
        )
        # server gerçek portu "SKILLS_MCP_LISTENING port=N" satırıyla bildirir
        line = self.proc.stdout.readline()
        self.assertIn("SKILLS_MCP_LISTENING", line)
        self.port = int(line.rsplit("port=", 1)[1].strip())

    def tearDown(self):
        self.proc.terminate()
        self.proc.wait(timeout=10)
        self.proc.stdout.close()
        self.tmp.cleanup()

    def post(self, payload):
        req = urllib.request.Request(
            f"http://127.0.0.1:{self.port}/mcp",
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            return json.loads(resp.read().decode("utf-8"))

    def test_http_initialize_and_pinned_call(self):
        r = self.post({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                       "params": {"protocolVersion": "2024-11-05",
                                  "capabilities": {},
                                  "clientInfo": {"name": "t", "version": "0"}}})
        self.assertEqual(r["result"]["serverInfo"]["name"], "skills-mcp")

        r = self.post({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                       "params": {"name": "skills_list", "arguments": {}}})
        text = r["result"]["content"][0]["text"]
        self.assertIn("demo", text)
        self.assertNotIn("solo", text)

        r = self.post({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                       "params": {"name": "skill_load", "arguments": {"name": "demo"}}})
        self.assertIn("demo v1 body", r["result"]["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
