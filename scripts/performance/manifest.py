"""Independent SHA-256 source inventory; IDs hash relative path bytes, no raw names."""
import hashlib, os, stat
from pathlib import Path

def manifest(root):
    root = Path(root)
    if root.is_symlink() or not root.is_dir(): raise ValueError('Owned fixture directory required')
    entries = []
    def visit(directory):
        for item in sorted(os.scandir(directory), key=lambda item: os.fsencode(item.name)):
            path = Path(item.path)
            info = path.lstat()
            relative = os.fsencode(os.path.relpath(path, root))
            entry = {'id': hashlib.sha256(relative).hexdigest(), 'mode': info.st_mode,
                     'size': info.st_size, 'mtimeNs': info.st_mtime_ns, 'inode': info.st_ino,
                     'device': info.st_dev, 'uid': info.st_uid, 'gid': info.st_gid}
            if stat.S_ISREG(info.st_mode):
                digest = hashlib.sha256()
                with path.open('rb') as source:
                    for chunk in iter(lambda: source.read(65536), b''): digest.update(chunk)
                entry['sha256'] = digest.hexdigest()
            elif stat.S_ISLNK(info.st_mode):
                entry['linkSha256'] = hashlib.sha256(os.fsencode(os.readlink(path))).hexdigest()
            elif stat.S_ISDIR(info.st_mode): visit(path)
            else: raise ValueError('Unexpected special fixture file')
            entries.append(entry)
    visit(root)
    entries.sort(key=lambda item: item['id'])
    return {'algorithm': 'sha256', 'entries': entries, 'coverage': 'names, types, bytes, mode, mtime, inode/device, uid/gid, link target; excludes atime and ACL/xattr/streams'}

if __name__ == '__main__':
    import json, sys
    print(json.dumps(manifest(sys.argv[1])))
