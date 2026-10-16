#!/usr/bin/env python3
"""Prepare the public build locally using the headquarters signing/notarization gate; no publication."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path.home() / 'Dev/tools/dev/lib/tools/macapp'))
import distribution_grade

source = ROOT / 'build/水利工具箱.app'
target = ROOT / 'build/release/水利工具箱.app'
info = plistlib.loads((source / 'Contents/Info.plist').read_bytes())
assert info['CFBundleIdentifier'] == 'io.github.zengtianli.HydroMac'
stage = ROOT / 'build/release'; stage.mkdir(parents=True, exist_ok=True)
if target.exists():
    old = plistlib.loads((target / 'Contents/Info.plist').read_bytes())
    assert old['CFBundleIdentifier'] == info['CFBundleIdentifier']
    retired = stage / ('previous-' + str(old['CFBundleVersion']) + '.app')
    assert not retired.exists(), 'Previous output already retained; read original before retry'
    target.rename(retired)
subprocess.run(['ditto', str(source), str(target)], check=True)
facts = distribution_grade.sign(target, identity='Developer ID Application: tianli Zeng (B9LJH93LA4)')
(stage / 'signature.json').write_text(json.dumps(facts, ensure_ascii=False, indent=2) + '\n')
facts = distribution_grade.notarize(target, timeout='20m')
(stage / 'notarization.json').write_text(json.dumps(facts, ensure_ascii=False, indent=2) + '\n')
files = {str(p.relative_to(target)): ('link:' + os.readlink(p) if p.is_symlink() else hashlib.sha256(p.read_bytes()).hexdigest())
         for p in target.rglob('*') if p.is_file() or p.is_symlink()}
(stage / 'files.json').write_text(json.dumps(files, ensure_ascii=False, sort_keys=True, indent=2) + '\n')
archive = stage / ('HydroMac-' + info['CFBundleShortVersionString'] + '-arm64.zip')
subprocess.run(['ditto', '-c', '-k', '--keepParent', str(target), str(archive)], check=True)
(stage / 'package.json').write_text(json.dumps({'version': info['CFBundleShortVersionString'], 'build': info['CFBundleVersion'],
    'bundle_id': info['CFBundleIdentifier'], 'filename': archive.name, 'bytes': archive.stat().st_size,
    'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(), 'notarized': True, 'published': False}, indent=2) + '\n')
print('Current public bundle signed, notarized, stapled and packaged on Mini; no upload', flush=True)
