"""Owned M5 fixtures and independent SHA256 manifests; never takes a library path."""
import argparse, hashlib, json, platform, re, shutil, struct, subprocess, sys, urllib.parse, uuid, zlib
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'performance'))
from manifest import manifest

parser=argparse.ArgumentParser()
parser.add_argument('--camera-samples',action='store_true',help='Fetch six pinned CC0 camera fixtures from raw.pixls.us')
parser.add_argument('--reuse-fixtures',help='Read a previously owned fixture UUID without downloading or changing its sources')
parser.add_argument('--packaged',action='store_true',help='Exercise the fixed locally built app worker/helper')
args=parser.parse_args()
if platform.system()!='Darwin': raise SystemExit('Native ImageIO fixture adapter is macOS-only; no Windows claim')
if shutil.disk_usage('.').free<4*1024**3: raise SystemExit('Four GiB reserve required; no cleanup attempted')
run=str(uuid.uuid4());owned=Path('.context/formats')/run
if Path('.context').is_symlink() or owned.parent.is_symlink(): raise SystemExit('Owned namespace must not be a link')
owned.mkdir(parents=True,exist_ok=False);sources=owned/'sources';sources.mkdir()
(owned/'ownership.json').write_text(json.dumps({'run':run,'version':1,'kind':'m5-format-fixture'})+'\n')
if args.reuse_fixtures:
    previous=owned.parent/str(uuid.UUID(args.reuse_fixtures))
    marker=json.loads((previous/'ownership.json').read_text())
    assert marker['run']==previous.name and marker['kind']=='m5-format-fixture' and not previous.is_symlink()
    expected=json.loads((previous/'manifest-after.json').read_text())
    origin=str(uuid.UUID(marker.get('sourceRun',previous.name)))
    source_owner=owned.parent/origin
    origin_marker=json.loads((source_owner/'ownership.json').read_text())
    assert origin_marker['run']==origin and origin_marker['kind']=='m5-format-fixture' and not source_owner.is_symlink()
    sources=source_owner/'sources'
    if sources.is_symlink() or manifest(sources)!=expected: raise SystemExit('Retained source identity changed; no reuse')
    (owned/'ownership.json').write_text(json.dumps({'run':run,'version':1,'kind':'m5-format-fixture','sourceRun':origin})+'\n')
    fixtures=json.loads((previous/'fixtures.json').read_text())
    for item in fixtures:
        if item['file'].startswith('generated'): item['id']=item['file'].replace('.','-').lower()
else:
    fixtures=[]
    def item(name, expected='ready', **extra):
        fixtures.append({'id':name.replace('.','-').lower() if name.startswith('generated') else name.rsplit('.',1)[0],'file':name,'expected':expected,**extra})
    def chunk(kind,data):
        return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    # Known asymmetric sRGB color blocks; no external image-generation dependency.
    w,h=96,48
    pixels=b''.join(b'\0'+b''.join(bytes((220,30,20) if x<w//2 and y<h//2 else (20,180,40) if y<h//2 else (20,40,210) if x<w//2 else (230,190,20)) for x in range(w)) for y in range(h))
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(pixels))+chunk(b'IEND',b'')
    (sources/'generated.png').write_bytes(png);item('generated.png')
    for extension,codec in [('jpg','jpeg'),('heic','heic'),('tif','tiff'),('gif','gif'),('bmp','bmp')]:
        subprocess.run(['/usr/bin/sips','-s','format',codec,str(sources/'generated.png'),'--out',str(sources/f'generated.{extension}')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        item(f'generated.{extension}')
    # HEIF aliases and case-insensitive discovery use the same actual HEIC container.
    for name in ['generated.heif','generated-upper.HEIC']:
        (sources/name).write_bytes((sources/'generated.heic').read_bytes());item(name)
    # A real tagged Display-P3 fixture made from known sRGB patch values.
    srgb='/System/Library/ColorSync/Profiles/sRGB Profile.icc'
    p3='/System/Library/ColorSync/Profiles/Display P3.icc'
    subprocess.run(['/usr/bin/sips','--embedProfile',srgb,str(sources/'generated.png'),'--out',str(owned/'tagged-srgb.png')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    subprocess.run(['/usr/bin/sips','--matchTo',p3,str(owned/'tagged-srgb.png'),'--out',str(sources/'generated-p3.png')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    item('generated-p3.png')
    (sources/'malformed.heic').write_bytes(b'not a HEIC');item('malformed.heic','unavailable')
    (sources/'over-input.heic').write_bytes(b'\0'*(32*1024*1024+1));item('over-input.heic','unavailable')
    # Legitimate encoded tiny image whose declared width exceeds the adapter limit.
    w2=12001
    wide=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w2,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+b'\xff\0\0'*w2))+chunk(b'IEND',b'')
    (sources/'over-dimension.png').write_bytes(wide);item('over-dimension.png','unavailable')
    if args.camera_samples:
        for sample in json.loads(Path('scripts/formats/camera-fixtures.json').read_text()):
            assert re.fullmatch(r'[a-z0-9-]{1,64}',sample['id']) and re.fullmatch(r'[a-f0-9]{64}',sample['sha256']) and sample['extension'] in ['cr2','cr3','nef','arw','dng','raf']
            url=sample['url'];parsed=urllib.parse.urlparse(url)
            assert parsed.scheme=='https' and parsed.netloc=='raw.pixls.us' and sample['license']=='CC0-1.0'
            # System curl uses the host trust store; never disable TLS verification.
            downloaded=owned/f'download-{sample["id"]}'
            subprocess.run(['/usr/bin/curl','--fail','--location','--silent','--show-error','--globoff','--proto','=https','--proto-redir','=https','--max-time','120','--max-filesize',str(32*1024*1024),'--output',str(downloaded),urllib.parse.quote(url,safe=':/')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            data=downloaded.read_bytes()
            if len(data)>32*1024*1024: raise SystemExit('Camera fixture exceeds retained input budget; no limit relaxed')
            if hashlib.sha256(data).hexdigest()!=sample['sha256']: raise SystemExit('Camera fixture identity mismatch; stop without decoding')
            name=f'{sample["id"]}.{sample["extension"]}'
            (sources/name).write_bytes(data);item(name,source=sample)
(owned/'fixtures.json').write_text(json.dumps(fixtures,indent=2)+'\n')
before=manifest(sources);(owned/'manifest-before.json').write_text(json.dumps(before,indent=2)+'\n')
command=['target/release/examples/format_probe','--run',run]+(['--packaged'] if args.packaged else [])
try:
    process=subprocess.run(command,capture_output=True,timeout=300)
    if process.returncode: raise RuntimeError('Owned format probe failed; raw private parser output not exported')
    result=json.loads(process.stdout)
except Exception:
    (owned/'failure.json').write_text(json.dumps({'failed':True})+'\n');raise
finally:
    after=manifest(sources);(owned/'manifest-after.json').write_text(json.dumps(after,indent=2)+'\n')
    if before!=after: raise RuntimeError('Independent source manifest mismatch; stop')
result.update(run=run,sourceRun=sources.parent.name,sourceManifestEqual=True,sourceEntries=len(before['entries']),platform=platform.platform(),packaged=args.packaged,
              binarySha256=hashlib.file_digest(open('target/release/examples/format_probe','rb'),'sha256').hexdigest())
worker=Path('target/release/bundle/macos/Stillharbour.app/Contents/MacOS/stillharbour' if args.packaged else 'target/release/stillharbour')
result['workerSha256']=hashlib.file_digest(worker.open('rb'),'sha256').hexdigest()
result['sdk']=subprocess.check_output(['/usr/bin/xcrun','--show-sdk-version'],text=True).strip()
(owned/'results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'run':run,'sourceManifestEqual':True,'results':result['results']}))
if any(item['status']!='passed' for item in result['results']): raise SystemExit(1)
