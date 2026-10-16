#!/usr/bin/env python3
"""Resume the queued product build, verified installation, fixed acceptance and raw native capture.

No retry loop, process termination, publication or bypass of the shared build queue.
"""
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / 'build/release/水利工具箱.app'
INSTALLED = Path('/Applications/水利工具箱.app')
RECEIPT = ROOT / 'perf/build-receipt.json'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

def run(words):
    done = subprocess.run(list(map(str, words)), cwd=ROOT)
    if done.returncode: raise SystemExit(done.returncode)

def current():
    try:
        receipt = json.loads(RECEIPT.read_text()); a = receipt['artifact']
        info = plistlib.loads((APP / 'Contents/Info.plist').read_bytes())
        return (info['CFBundleIdentifier'] == a['bundle_id'] == 'io.github.zengtianli.HydroMac'
                and info['CFBundleShortVersionString'] == a['version'] and info['CFBundleVersion'] == a['build']
                and sha(APP / a['executable']) == a['sha256']
                and all(sha(ROOT / p) == h for p, h in receipt['source']['files'].items()))
    except (OSError, KeyError, ValueError): return False

if not current():
    # An existing queue record is retained. This is one worker step, not a new unbounded build.
    run(['chapter', 'work', '--once', '--app', 'hydro-mac-oss', '--json'])
    if not current():
        print('当前源码的签名构建尚未就绪；保留原队列，等待原调度，不安装旧候选。', file=sys.stderr)
        raise SystemExit(75)
receipt = json.loads(RECEIPT.read_text()); a = receipt['artifact']
already = False
try:
    i = plistlib.loads((INSTALLED / 'Contents/Info.plist').read_bytes())
    already = (i['CFBundleIdentifier'] == a['bundle_id'] and i['CFBundleVersion'] == a['build']
               and i['CFBundleShortVersionString'] == a['version'] and sha(INSTALLED / a['executable']) == a['sha256'])
except (OSError, KeyError, ValueError): pass
if not already:
    run(['bash', 'build.sh', '--install', '--prepared', APP])
for check in ['functionality', 'recovery', 'privacy', 'native_ui', 'agent_cli', 'cli_entry', 'product_basics']:
    run(['chapter', 'sop', 'accept', '--app', 'hydro-mac-oss', '--check', check, '--json'])
if not already:
    run(['chapter', 'sop', 'accept', '--app', 'hydro-mac-oss', '--check', 'installed_icon', '--json'])
out = ROOT / 'build' / ('media-current-' + time.strftime('%Y%m%d-%H%M%S'))
run([sys.executable, ROOT / 'scripts/media/capture.py', '--out', out])
print(json.dumps({'app': str(INSTALLED), 'version': a['version'], 'build': a['build'],
                  'media_plan': str(out / 'plan.json'), 'next': '原共享 demo_video.py audio/render/verify，逐关键帧核可读性后再沿既有发行入口发布。'}, ensure_ascii=False))
