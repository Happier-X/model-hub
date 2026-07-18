#!/usr/bin/env python3
"""真实 octopus 冒烟（独立测试端口，不按进程名杀全机 octopus）。"""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("MODEL_HUB_GATEWAY_BIN", ROOT / "tools" / "octopus" / "octopus.exe"))
PORT = int(os.environ.get("MODEL_HUB_E2E_PORT", "18090"))
DATA = ROOT / "tools" / "octopus" / "testdata-smoke"


def wait_port(port: int, timeout: float = 20.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), 0.3):
                return True
        except OSError:
            time.sleep(0.15)
    return False


def free_port(port: int) -> None:
    """仅结束占用该端口的进程，不按名称杀 octopus。"""
    script = (
        f"$c = Get-NetTCPConnection -LocalPort {port} -ErrorAction SilentlyContinue | "
        "Select-Object -First 1; "
        "if ($null -ne $c) { "
        "  Stop-Process -Id $c.OwningProcess -Force -ErrorAction SilentlyContinue; "
        "  Write-Output ('killed ' + $c.OwningProcess) "
        "} else { Write-Output 'free' }"
    )
    subprocess.run(
        ["powershell", "-Command", script],
        check=False,
        capture_output=True,
        text=True,
    )


def http(method: str, path: str, body: dict | None = None, token: str | None = None):
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers: dict[str, str] = {}
    if body is not None:
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(
        f"http://127.0.0.1:{PORT}{path}",
        data=data,
        headers=headers,
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            payload = raw
        return exc.code, payload


def main() -> int:
    if not BIN.is_file():
        print(f"MISSING_BIN {BIN}")
        print("Run: pnpm prepare:octopus  # or scripts/prepare-bundled-octopus.ps1")
        return 2

    free_port(PORT)
    if DATA.exists():
        shutil.rmtree(DATA, ignore_errors=True)
    (DATA / "data").mkdir(parents=True)
    (DATA / "data" / "config.json").write_text(
        json.dumps(
            {
                "server": {"host": "127.0.0.1", "port": PORT},
                "database": {"type": "sqlite", "path": "data/data.db"},
                "log": {"level": "info"},
            },
            indent=2,
        ),
        encoding="utf-8",
    )

    env = os.environ.copy()
    env.update(
        {
            "OCTOPUS_SERVER_HOST": "127.0.0.1",
            "OCTOPUS_SERVER_PORT": str(PORT),
            "OCTOPUS_DATABASE_TYPE": "sqlite",
            "OCTOPUS_DATABASE_PATH": "data/data.db",
            "OCTOPUS_LOG_LEVEL": "info",
        }
    )
    proc = subprocess.Popen(
        [str(BIN), "start", "--config", "data/config.json"],
        cwd=str(DATA),
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        if not wait_port(PORT):
            print("FAIL start/listen")
            return 1
        print("OK listen", PORT)

        code, login = http(
            "POST",
            "/api/v1/user/login",
            {"username": "admin", "password": "admin", "expire": 86400},
        )
        if code != 200 or not isinstance(login, dict):
            print("FAIL login", code, login)
            return 1
        token = login.get("data", {}).get("token")
        if not token:
            print("FAIL token")
            return 1
        print("OK login")

        code, _ = http("GET", "/api/v1/channel/list", token=token)
        print("OK channel_list" if code == 200 else f"FAIL channel_list {code}")

        body = {
            "name": "smoke-openai",
            "type": 0,
            "enabled": True,
            "base_urls": [{"url": "https://api.openai.com/v1", "delay": 0}],
            "keys": [{"enabled": True, "channel_key": "sk-test-fake", "remark": "smoke"}],
            "model": "gpt-4o-mini",
            "custom_model": "",
            "proxy": False,
            "auto_sync": False,
            "auto_group": 0,
            "custom_header": [],
        }
        code, created = http("POST", "/api/v1/channel/create", body, token)
        if code != 200:
            print("FAIL channel_create", code, created)
            return 1
        print("OK channel_create")

        code, ch_list = http("GET", "/api/v1/channel/list", token=token)
        channels = (ch_list or {}).get("data") or [] if isinstance(ch_list, dict) else []
        channel_id = channels[0]["id"] if channels else None
        if channel_id is None:
            print("FAIL no channel id")
            return 1

        gbody = {
            "name": "smoke-group",
            "mode": 1,
            "match_regex": "",
            "items": [
                {
                    "channel_id": channel_id,
                    "model_name": "gpt-4o-mini",
                    "priority": 1,
                    "weight": 1,
                }
            ],
        }
        code, _ = http("POST", "/api/v1/group/create", gbody, token)
        if code != 200:
            print("FAIL group_create", code)
            return 1
        print("OK group_create")

        code, _ = http("GET", "/api/v1/group/list", token=token)
        print("OK group_list" if code == 200 else f"FAIL group_list {code}")
        code, _ = http("GET", "/api/v1/log/list?page=1&page_size=5", token=token)
        print("OK log_list" if code == 200 else f"FAIL log_list {code}")

        # 管理 JWT 创建网关客户端 API Key，再探测 /v1 鉴权闭环
        code, key_resp = http(
            "POST",
            "/api/v1/apikey/create",
            {"name": "smoke-client", "enabled": True},
            token=token,
        )
        if code != 200 or not isinstance(key_resp, dict):
            print("FAIL apikey_create", code, key_resp)
            return 1
        gateway_key = (key_resp.get("data") or {}).get("api_key")
        if not gateway_key or not str(gateway_key).startswith("sk-octopus-"):
            print("FAIL apikey_create missing sk-octopus key", key_resp)
            return 1
        print("OK apikey_create")

        code, _ = http("GET", "/api/v1/apikey/list", token=token)
        print("OK apikey_list" if code == 200 else f"FAIL apikey_list {code}")

        # 错误占位 Key 应 401（证明不是免鉴权）
        bad_code, _ = http("GET", "/v1/models", token="sk-placeholder")
        if bad_code != 401:
            print(f"FAIL v1_models_bad_key expected 401 got {bad_code}")
            return 1
        print("OK v1_models_bad_key 401")

        # 正确网关 Key：期望非 401（200 或业务空列表均可）
        models_code, models_body = http("GET", "/v1/models", token=str(gateway_key))
        if models_code == 401:
            print("FAIL v1_models with gateway key still 401", models_body)
            return 1
        print(f"OK v1_models status={models_code}")

        # 可选：无真实上游时 Chat 可为业务错误，但不能是鉴权失败
        chat_code, chat_body = http(
            "POST",
            "/v1/chat/completions",
            {
                "model": "smoke-group",
                "messages": [{"role": "user", "content": "ping"}],
            },
            token=str(gateway_key),
        )
        if chat_code == 401:
            print("FAIL chat auth 401", chat_body)
            return 1
        print(f"OK v1_chat_auth_ok status={chat_code} (业务错误可接受)")

        print("SMOKE_PASS")
        return 0
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()
        free_port(PORT)


if __name__ == "__main__":
    sys.exit(main())
