#!/usr/bin/env python3
"""Queue-owned build/test entry; current-source candidate is reused only after full receipt verification."""
import os
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
import app_sop
sys.path.insert(0, str(Path.home() / 'Dev/tools/dev/lib/tools/macapp/ios'))
import sim_lane
app = app_sop.load_apps('hydro-mac-oss')[0]
artifact = ROOT / 'build/release/水利工具箱.app'
receipt = ROOT / 'perf/build-receipt.json'
patterns = ['Sources/**', 'build.sh', 'scripts/*.py', 'scripts/*.sh', 'scripts/hydromac', 'scripts/accept/*.py', 'cli/src/**', 'cli/data/**', 'cli/Cargo.toml', 'cli/Cargo.lock', 'icon/AppIcon.icns']
valid, reason = app_sop.verify_build_receipt(app, artifact, receipt_path=receipt)
if not valid:
    print('Current source build: ' + reason, flush=True)
    if receipt.exists():
        archive = ROOT / 'build/accept/before-release-build-receipt.json'
        if not archive.exists(): archive.write_bytes(receipt.read_bytes())
    try:
        # This bare swiftc/cargo product has no Xcode scheme for sim_lane.build.
        # Compile in its shared parallel slot; sampling/boot quiet gates remain separate.
        with sim_lane.BuildSlot('hydro-mac-oss custom Release build', wait=0):
            app_sop.build_receipt(app, artifact, patterns, 'BUILD_ONLY=1 bash build.sh && PYTHONDONTWRITEBYTECODE=1 python3 scripts/prepare_release.py', output_path=receipt)
    except sim_lane.Busy as error:
        print('Build deferred: ' + str(error), file=sys.stderr, flush=True)
        raise SystemExit(75)
env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', HYDROMAC_ACCEPT_APP=str(artifact), HYDROMAC_ACCEPT_RECEIPT=str(receipt), SOP_OUT_DIR=str(ROOT / 'build/accept/unit'))
for check in ['functionality', 'recovery', 'privacy']:
    result = subprocess.run([sys.executable, str(ROOT / 'scripts/accept/product_checks.py'), '--check', check], cwd=ROOT, env=env)
    if result.returncode: raise SystemExit(result.returncode)
print('Product tests: all passed', flush=True)
