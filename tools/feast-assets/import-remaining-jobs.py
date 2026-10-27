"""Import native metadata from verified cached pre-April 2022 guides/6.08 sheet.

Gameplay implementations are hand-written in AdditionalJobs.lua, not inferred
from prose. This script deliberately fails if a guide action has no native row.
"""
import csv, html, json, pathlib, re
root = pathlib.Path('.')
jobs = {20:'monk',30:'ninja',23:'bard',31:'machinist',38:'dancer',27:'summoner',35:'redmage',28:'scholar',33:'astrologian',32:'darkknight',37:'gunbreaker',39:'reaper',40:'sage'}
with open('target/feast-whm/Action-6.08.csv', encoding='utf-8') as f:
    reader=csv.reader(f); next(reader); headers=next(reader); next(reader)
    rows=[dict(zip(headers,r)) for r in reader]
catalog=json.loads((root/'resources/feast-actions.json').read_text())
catalog=[r for r in catalog if r['job'] not in jobs]
data=[]
def clean(s): return re.sub(r'\s+',' ',html.unescape(re.sub('<[^>]+>',' ',s))).strip()
for job,slug in jobs.items():
    page=(root/f'target/feast-more-jobs/{slug}.html').read_text(encoding='utf-8-sig')
    for block in re.findall(r'<tr id="pvp_action__.*?</tr>',page,re.S):
        name=clean(re.search(r'<strong>(.*?)</strong>',block,re.S)[1])
        row=next(x for x in rows if x['Name']==name and x['ClassJob']==str(job) and x['IsPvP']=='True')
        description=clean(re.search(r'<td class="content">(.*?)</td>',block,re.S)[1]).split('※')[0].strip()
        key=re.sub('[^a-z0-9_]', '', name.lower().replace(' ','_').replace('-','_'))
        # Avoid same-name actions with different historical job IDs.
        if any(a['key']==key and a['id']!=int(row['#']) for a in catalog): key=slug+'_'+key
        record=dict(job=job,key=key,id=int(row['#']),icon=int(row['Icon']),name=name,cast=int(row['Cast<100ms>'])/10,recast=int(row['Recast<100ms>'])/10,group=int(row['CooldownGroup'])-1,secondary=int(row['AdditionalCooldownGroup'])-1,charges=max(1,int(row['MaxCharges'])),range=int(row['Range']),radius=int(row['EffectRange']),description=description,category={'2':'Spell','3':'Weaponskill'}.get(row['ActionCategory'],'Ability'),mpCost=int(row['PrimaryCost{Value}']) if row['PrimaryCost{Type}']=='51' else int(row['PrimaryCost{Value}'])*100 if row['PrimaryCost{Type}']=='3' else 0)
        catalog.append(record)
        target='ground' if row['TargetArea']=='True' else 'enemy' if row['CanTargetHostile']=='True' else 'ally' if row['CanTargetParty']=='True' else 'self'
        data.append(dict(record,target=target,combo=int(row['Action{Combo}'])))
    role=19 if job in (32,37) else 34 if job in (20,30,39) else 25 if job in (27,35) else 24 if job in (28,33,40) else 23
    shared={19:{'low_blow','full_swing','weapon_throw','reprisal','rampart','retaliation','aegis_boon'},34:{'smite','feint','bloodbath','arms_length','fetter_ward','backstep','raw_destruction'},25:{'phantom_dart','drain','addle','aetheric_burst','swiftcast','manaward','cometeor'},24:{'purify','repose','protect','rescue','largesse','attunement','rain'}}
    if role!=23:
        catalog.extend(dict(a,job=job) for a in catalog[:] if a['job']==role and a['key'] in shared[role]|{'bolt','kit'})
    # WHM has a separate catalog, so derive its shared native metadata directly.
    if role==24:
        for key,aid in [('purify',1584),('repose',17690),('protect',17832),('rescue',17688),('largesse',18957),('attunement',17689),('rain',3362),('bolt',18942),('kit',18943)]:
            if any(a['job']==job and a['id']==aid for a in catalog): continue
            r=next(x for x in rows if x['#']==str(aid))
            catalog.append(dict(job=job,key=key,id=aid,icon=int(r['Icon']),name=r['Name'],cast=int(r['Cast<100ms>'])/10,recast=int(r['Recast<100ms>'])/10,group=int(r['CooldownGroup'])-1,secondary=int(r['AdditionalCooldownGroup'])-1,charges=max(1,int(r['MaxCharges'])),range=int(r['Range']),radius=int(r['EffectRange']),description='',category='Ability',mpCost=0))
# Ranged role actions have no existing kit to clone.
page=(root/'target/feast-more-jobs/bard.html').read_text(encoding='utf-8-sig')
for block in re.findall(r'<tr id="(?:additional_action__|pvpcommmononlyaction_|limitbreak_action__).+?</tr>',page,re.S):
    name=clean(re.search(r'<strong>(.*?)</strong>',block,re.S)[1]) if '<strong>' in block else clean(re.search(r'<p class="skill">(.*?)</p>',block,re.S)[1])
    r=next(x for x in rows if x['Name']==name and x['IsPvP']=='True')
    key={'Medical Kit':'kit'}.get(name,re.sub('[^a-z0-9_]', '',name.lower().replace(' ','_')))
    record=dict(job=23,key=key,id=int(r['#']),icon=int(r['Icon']),name=name,cast=int(r['Cast<100ms>'])/10,recast=int(r['Recast<100ms>'])/10,group=int(r['CooldownGroup'])-1,secondary=int(r['AdditionalCooldownGroup'])-1,charges=max(1,int(r['MaxCharges'])),range=int(r['Range']),radius=int(r['EffectRange']),description=clean(re.search(r'<td class="content">(.*?)</td>',block,re.S)[1]),category='Ability',mpCost=0)
    for job in (23,31,38): catalog.append(dict(record,job=job))
    if key not in ('bolt','kit'):
        data.append(dict(record,target='enemy' if r['CanTargetHostile']=='True' else 'self',combo=0))
(root/'resources/feast-actions.json').write_text(json.dumps(catalog,indent=2)+'\n',encoding='utf-8')
def lua(v):
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,bool): return 'true' if v else 'false'
    return str(v)
(root/'resources/scripts/feast/AdditionalActionData.lua').write_text('-- Generated by tools/feast-assets/import-remaining-jobs.py; native 6.08 metadata.\nreturn {\n'+''.join(' ['+lua(d['key'])+']={'+','.join(k+'='+lua(d[k]) for k in ('job','id','cast','recast','charges','range','radius','category','mpCost','target','combo','group','secondary'))+'},\n' for d in data)+'}\n',encoding='utf-8')
print('Imported',len(data),'job-specific actions')
