#!/usr/bin/env python3
"""Rebuild bundled offline place data from explicitly supplied, pinned sources.

No network access or archive-tree extraction. See assets/places/manifest.json.
"""
import argparse
import hashlib
import gzip
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / 'assets/places'


def read_source(directory, name, record):
    path = directory / name
    if path.is_symlink() or path.stat().st_size != record['bytes'] or record['bytes'] > 30_000_000:
        raise SystemExit(f'Invalid pinned place source: {name}')
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != record['sha256']:
        raise SystemExit(f'Changed pinned place source: {name}')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', type=Path, required=True)
    args = parser.parse_args()
    if OUTPUT.is_symlink() or not OUTPUT.resolve().is_relative_to(ROOT):
        raise SystemExit('Place output escaped the workspace.')
    for name in ('manifest.json', 'NOTICE.md', 'cities.json.gz', 'countries.json.gz'):
        if (OUTPUT / name).is_symlink():
            raise SystemExit('Refusing a linked place output.')
    manifest = json.loads((OUTPUT / 'manifest.json').read_text())
    sources = {name: read_source(args.sources, name, record)
               for name, record in manifest['sources'].items()}
    country_names = {}
    country_codes = {}
    for line in sources['countryInfo.txt'].decode('utf-8').splitlines():
        if line and not line.startswith('#'):
            fields = line.split('\t')
            country_names[fields[0]] = fields[4]
            country_codes[fields[0]] = [fields[0], fields[1], fields[3]]
    regions = {fields[0]: fields[1] for fields in
               (line.split('\t') for line in sources['admin1CodesASCII.txt'].decode('utf-8').splitlines())}
    # Read only the fixed text member. Never extract arbitrary archive paths.
    import io
    with zipfile.ZipFile(io.BytesIO(sources['cities15000.zip'])) as archive:
        member = archive.getinfo('cities15000.txt')
        if member.file_size > 50_000_000:
            raise SystemExit('Place source member exceeds its limit.')
        lines = archive.read(member).decode('utf-8').splitlines()
    cities = []
    for line in lines:
        f = line.split('\t')
        if len(f) != 19 or f[6] != 'P':
            raise SystemExit('Unexpected GeoNames city row.')
        latitude, longitude = float(f[4]), float(f[5])
        if not (-90 <= latitude <= 90 and -180 <= longitude <= 180):
            raise SystemExit('Invalid city coordinate.')
        aliases = sorted(set([f[2], *f[3].split(','), *country_codes.get(f[8], [])]) - {'', f[1]})
        cities.append({'id': f'places-v1:city:{int(f[0])}', 'name': f[1],
                       'country': country_names.get(f[8], f[8]),
                       'region': regions.get(f'{f[8]}.{f[10]}', ''),
                       'latitude': latitude, 'longitude': longitude,
                       'population': int(f[14]), 'aliases': aliases})
    countries = []
    for feature in json.loads(sources['countries.geojson'])['features']:
        props, geometry = feature['properties'], feature['geometry']
        if geometry['type'] == 'MultiPolygon':
            polygons = geometry['coordinates']
        elif geometry['type'] == 'Polygon':
            polygons = [geometry['coordinates']]
        else:
            raise SystemExit('Unexpected country geometry.')
        for polygon in polygons:
            for ring in polygon:
                if len(ring) < 4 or ring[0] != ring[-1]:
                    raise SystemExit('Unclosed country ring.')
                for point in ring:
                    if len(point) != 2 or not (-180 <= point[0] <= 180 and -90 <= point[1] <= 90):
                        raise SystemExit('Invalid country coordinate.')
        aliases = sorted({value for key, value in props.items()
                          if (key.startswith('NAME') or key in ('ADMIN', 'FORMAL_EN', 'ABBREV', 'ISO_A2_EH', 'ADM0_A3'))
                          and isinstance(value, str) and value and value != '-99'})
        aliases = sorted(set(aliases + country_codes.get(props.get('ISO_A2_EH'), [])) - {''})
        countries.append({'id': f'places-v1:country:{props["ADM0_A3"]}',
                          'name': props.get('NAME_EN') or props['ADMIN'],
                          'aliases': aliases, 'polygons': polygons})
    if len({city['id'] for city in cities}) != len(cities) or len({country['id'] for country in countries}) != len(countries):
        raise SystemExit('Repeated place identity.')
    for name, records in [('cities.json.gz', cities), ('countries.json.gz', countries)]:
        data = (json.dumps(sorted(records, key=lambda item: item['id']),
                           ensure_ascii=False, separators=(',', ':')) + '\n').encode('utf-8')
        data = gzip.compress(data, mtime=0)
        (OUTPUT / name).write_bytes(data)
        manifest['files'][f'assets/places/{name}'] = hashlib.sha256(data).hexdigest()
    manifest['files'] = {path: value for path, value in manifest['files'].items() if path.endswith('.gz')}
    manifest['files']['assets/places/NOTICE.md'] = hashlib.sha256((OUTPUT / 'NOTICE.md').read_bytes()).hexdigest()
    manifest['counts'] = {'cities': len(cities), 'countriesAndTerritories': len(countries)}
    (OUTPUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(f'Prepared {len(cities)} cities and {len(countries)} country/territory geometries.')


if __name__ == '__main__':
    main()
