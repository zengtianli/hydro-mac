#!/usr/bin/env python3
"""Queue-owned build/test entry; current-source candidate is reused only after full receipt verification."""
import os
import fcntl
import hashlib
import json
from contextlib import ExitStack
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
# Queue and foreground continuation share this product's fixed output directory.
# Exclude concurrent writers of that directory; other products keep their SDK slots.
(ROOT / 'build').mkdir(exist_ok=True)
product_build_guard = (ROOT / 'build/current-source-build.lock').open('a')
try:
    fcntl.flock(product_build_guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError:
    print('Build deferred: this product build/test entry is already running', file=sys.stderr, flush=True)
    raise SystemExit(75)
sys.path.insert(0, str(Path.home() / 'Dev/tools/dev/lib/tools/macapp/ios'))
import sim_lane
artifact = ROOT / 'build/release/水利工具箱.app'
receipt = ROOT / 'perf/build-receipt.json'
patterns = ['Sources/**', 'build.sh', 'scripts/*.py', 'scripts/*.sh', 'scripts/hydromac', 'scripts/accept/*.py', 'cli/src/**', 'cli/data/**', 'cli/Cargo.toml', 'cli/Cargo.lock', 'icon/AppIcon.icns']

try:
    recorded = json.loads(receipt.read_text())['source']['files']
    known_changes = not recorded or any(hashlib.sha256((ROOT / p).read_bytes()).hexdigest() != h for p, h in recorded.items())
except (OSError, KeyError, ValueError):
    known_changes = True
try:
    with ExitStack() as slots:
        # Reserve before the slower registry read when compilation is already necessary.
        # Cached tests take no SDK slot; the headquarters verifier remains authoritative.
        if known_changes:
            slots.enter_context(sim_lane.BuildSlot('hydro-mac-oss custom Release build', wait=0))
        sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
        import app_sop
        app = app_sop.load_apps('hydro-mac-oss')[0]
        valid, reason = app_sop.verify_build_receipt(app, artifact, receipt_path=receipt)
        if not valid:
            print('Current source build: ' + reason, flush=True)
            if not known_changes:
                slots.enter_context(sim_lane.BuildSlot('hydro-mac-oss custom Release build', wait=0))
            if receipt.exists():
                archive = ROOT / 'build/accept/before-release-build-receipt.json'
                if not archive.exists(): archive.write_bytes(receipt.read_bytes())
            # This bare swiftc/cargo product has no Xcode scheme for sim_lane.build.
            app_sop.build_receipt(app, artifact, patterns, 'BUILD_ONLY=1 bash build.sh && PYTHONDONTWRITEBYTECODE=1 python3 scripts/prepare_release.py', output_path=receipt)
except sim_lane.Busy as error:
    print('Build deferred: ' + str(error), file=sys.stderr, flush=True)
    raise SystemExit(75)
env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', HYDROMAC_ACCEPT_APP=str(artifact), HYDROMAC_ACCEPT_RECEIPT=str(receipt), SOP_OUT_DIR=str(ROOT / 'build/accept/unit'))
for check in ['functionality', 'recovery', 'privacy']:
    result = subprocess.run([sys.executable, str(ROOT / 'scripts/accept/product_checks.py'), '--check', check], cwd=ROOT, env=env)
    if result.returncode: raise SystemExit(result.returncode)
print('Product tests: all passed', flush=True)
