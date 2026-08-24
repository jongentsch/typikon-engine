"""Exercise the native Typikon ABI from a non-Rust consumer."""

from __future__ import annotations

import ctypes
import json
import pathlib
import sys


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: ffi_smoke.py PATH_TO_TYPIKON_SHARED_LIBRARY")

    library = ctypes.CDLL(sys.argv[1])
    library.typikon_compile.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    library.typikon_compile.restype = ctypes.c_void_p
    library.typikon_string_free.argtypes = [ctypes.c_void_p]
    library.typikon_string_free.restype = None

    fixture = (
        pathlib.Path(__file__).parent.parent
        / "crates"
        / "typikon-ffi"
        / "tests"
        / "fixtures"
        / "minimal-pack"
    )
    files = {
        path.relative_to(fixture).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(fixture.rglob("*.yaml"))
    }
    bundle = {
        "schema": "typikon.resource-bundle/v0.1",
        "files": files,
    }
    request = {
        "schema": "typikon.request/v0.1",
        "civil_date": "2026-07-25",
        "service": "great_vespers",
    }

    pointer = library.typikon_compile(
        json.dumps(bundle).encode("utf-8"),
        json.dumps(request).encode("utf-8"),
    )
    if not pointer:
        raise RuntimeError("typikon_compile returned a null pointer")
    try:
        response = json.loads(ctypes.string_at(pointer).decode("utf-8"))
    finally:
        library.typikon_string_free(pointer)

    if not response.get("ok"):
        raise RuntimeError(response.get("error", response))
    plan = response["plan"]
    assert plan["schema"] == "typikon.plan/v0.2"
    assert plan["pack"]["id"] == "ffi-test"
    assert plan["day"]["tone"] == "tone_7"
    print(json.dumps({"status": "ok", "pack": plan["pack"], "day": plan["day"]}))


if __name__ == "__main__":
    main()
