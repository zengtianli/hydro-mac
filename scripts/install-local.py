#!/usr/bin/env python3
"""Local iteration transaction for the public HydroMac bundle; never kills a process."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
PRODUCT = 'io.github.zengtianli.HydroMac'

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def info(path): return plistlib.loads((path / 'Contents/Info.plist').read_bytes())
def files(path):
    return {str(p.relative_to(path)): ('link:' + os.readlink(p) if p.is_symlink() else sha(p))
            for p in path.rglob('*') if p.is_file() or p.is_symlink()}
def call(words): return subprocess.run(list(map(str, words)), capture_output=True, text=True, check=True, timeout=60)
def signature(path):
    call(['codesign', '--verify', '--deep', '--strict', path])
    result = call(['codesign', '-dvv', path]); text = result.stdout + result.stderr
    authority = next((x.split('=', 1)[1] for x in text.splitlines() if x.startswith('Authority=Developer ID Application: ')), None)
    assert authority and 'runtime)' in text and '\nTimestamp=' in text, 'Stable Developer ID/runtime/timestamp required'
    requirement = call(['codesign', '-dr-', path])
    designated = next(x for x in (requirement.stdout + requirement.stderr).splitlines() if x.startswith('designated =>'))
    return authority, designated

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, default=ROOT / 'perf/build-receipt.json')
    args = parser.parse_args()
    source = args.app.resolve(); receipt_path = args.receipt.resolve()
    assert source.is_relative_to(ROOT / 'build') and not args.app.is_symlink(), 'Source must be this product build output'
    receipt = json.loads(receipt_path.read_text()); artifact = receipt['artifact']; expected = info(source)
    assert expected['CFBundleIdentifier'] == artifact['bundle_id'] == PRODUCT
    assert str(expected['CFBundleVersion']) == str(artifact['build']) and expected['CFBundleShortVersionString'] == artifact['version']
    assert sha(source / artifact['executable']) == artifact['sha256']
    assert all(sha(ROOT / p) == h for p, h in receipt['source']['files'].items()), 'Source changed after actual compilation'
    authority, requirement = signature(source)
    manifest = json.loads((source.parent / 'files.json').read_text())
    assert files(source) == manifest, 'Signed package differs from returned file manifest'
    roots = [Path('/Applications'), Path.home() / 'Applications']
    found = []
    for base in roots:
        for path in base.glob('*.app'):
            try:
                if info(path).get('CFBundleIdentifier') == PRODUCT: found.append(path)
            except (OSError, ValueError, plistlib.InvalidFileException): pass
    assert len(found) <= 1, 'Multiple public bundle installations; refuse ambiguous replacement'
    destination = found[0] if found else roots[0] / '水利工具箱.app'
    assert not destination.is_symlink()
    old_info = None
    if destination.exists():
        old_info = info(destination)
        assert old_info['CFBundleIdentifier'] == PRODUCT, 'Destination belongs to another product'
        old_authority, old_requirement = signature(destination)
        assert old_authority == authority and old_requirement == requirement, 'Developer identity/designated requirement changed'
    process_lines = call(['ps', '-axo', 'pid=,comm=']).stdout.splitlines()
    assert not any(line.strip().split(None, 1)[-1].startswith(str(destination) + '/Contents/MacOS/') for line in process_lines), 'Public product is in use; leave it untouched'
    stage = Path(tempfile.mkdtemp(prefix='.hydromac-local-', dir=destination.parent))
    candidate, previous = stage / 'Candidate.app', stage / 'Previous.app'
    call(['ditto', source, candidate]); assert files(candidate) == manifest
    assert signature(candidate) == (authority, requirement)
    swapped = False
    try:
        if destination.exists(): destination.rename(previous)
        candidate.rename(destination); swapped = True
        assert files(destination) == manifest and signature(destination) == (authority, requirement)
        assert info(destination) == expected
        call([destination / 'Contents/Resources/bin/hydromac', '--help'])
    except Exception:
        if swapped: destination.rename(stage / 'Rejected.app')
        if previous.exists(): previous.rename(destination)
        raise
    retired = None
    if previous.exists():
        trash = Path.home() / '.Trash' / ('hydromac-local-' + time.strftime('%Y%m%d-%H%M%S') + '-' + str(os.getpid()))
        trash.mkdir(parents=True); retired = trash / destination.name; previous.rename(retired)
    stage.rmdir()
    proof = {'schema_version': 1, 'mode': 'local_iteration', 'app': str(destination), 'bundle_id': PRODUCT,
             'version': artifact['version'], 'build': artifact['build'], 'main_sha256': sha(destination / artifact['executable']),
             'receipt': str(receipt_path), 'receipt_sha256': sha(receipt_path), 'identity': authority,
             'designated_requirement': requirement, 'all_files_match': files(destination) == manifest,
             'previous': old_info, 'retired_app': str(retired) if retired else None,
             'notarized': False, 'published': False, 'launched': False, 'processes_terminated': False}
    assessment = subprocess.run(['spctl', '-a', '-vv', str(destination)], capture_output=True, text=True, timeout=60)
    staple = subprocess.run(['xcrun', 'stapler', 'validate', str(destination)], capture_output=True, text=True, timeout=60)
    proof['notarized'] = assessment.returncode == 0 and 'source=Notarized Developer ID' in assessment.stderr and staple.returncode == 0
    proof['gatekeeper'] = assessment.stdout + assessment.stderr
    out = ROOT / 'build/install-product-basics-20261015/installed-proof.json'
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        saved = out.with_name('previous-proof-' + str(json.loads(out.read_text())['build']) + '.json')
        if not saved.exists(): saved.write_bytes(out.read_bytes())
    out.write_text(json.dumps(proof, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(proof, ensure_ascii=False))
    # Only the public product CLI is linked. An unrelated command or existing target is preserved.
    link = Path.home() / '.local/bin/hydromac'
    if not link.exists() and not link.is_symlink():
        link.parent.mkdir(parents=True, exist_ok=True)
        link.symlink_to(destination / 'Contents/Resources/bin/hydromac')

if __name__ == '__main__':
    main()
