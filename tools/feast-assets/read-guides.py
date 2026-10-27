"""Extract cached official historical PvP rows for implementation review."""
import html, json, pathlib, re
root = pathlib.Path('target/feast-more-jobs')
for path in root.glob('*.html'):
    page = path.read_text(encoding='utf-8-sig')
    entries = []
    for block in re.findall(r'<tr id="pvp_action__.*?</tr>', page, re.S):
        def clean(value):
            return re.sub(r'\s+', ' ', html.unescape(re.sub('<[^>]+>', ' ', value))).strip()
        name = clean(re.search(r'<strong>(.*?)</strong>', block, re.S)[1])
        content = clean(re.search(r'<td class="content">(.*?)</td>', block, re.S)[1])
        entries.append({'name': name, 'description': content})
    (root / (path.stem + '.json')).write_text(json.dumps(entries, indent=2), encoding='utf-8')
    print(path.stem, len(entries))
